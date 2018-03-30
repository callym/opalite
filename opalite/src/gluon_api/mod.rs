use std::{ ops::Deref, sync::{ Arc, Mutex } };
use anymap::{ any, Map as AnyMap };
use gluon::{
    self,
    vm::{
        self,
        api::{ Getable, Pushable, Userdata, ValueRef, VmType },
        gc::{ Gc, Move, Traverseable },
        thread::Context,
        vm::Value,
        Variants,
    },
    Thread,
};
use specs::{ self, Entities, Fetch, FetchMut, ReadStorage, System, VecStorage, WriteStorage };
use crate::{ Map, RLock };
use crate::opal::{ Gluon, GluonUi };
use crate::InitialPosition;

#[macro_use] pub mod macros;
pub mod prelude;
pub mod cgmath;
pub mod conrod;

register_gluon!(Map);
register_data!(Map);

impl<T> RLock<T> where T: Userdata + Clone {
    pub fn get_from_data(map: Data) -> Option<Self> {
        map.get()
    }

    pub fn insert_to_data(map: Data, data: Self) -> Option<Self> {
        map.insert(data)
    }

    pub fn remove_from_data(map: Data) -> Option<Self> {
        map.remove()
    }

    pub fn contains_in_data(map: Data) -> bool {
        map.contains::<Self>()
    }
}

impl<T> VmType for RLock<T> where T: Userdata + Clone {
    type Type = T;
}

impl<T> Traverseable for RLock<T> where T: Userdata + Clone {
    fn traverse(&self, _: &mut Gc) { }
}

impl<'vm, T> Getable<'vm> for RLock<T> where T: Userdata + Clone {
    fn from_value(vm: &'vm Thread, value: Variants) -> Self {
        match value.as_ref() {
            ValueRef::Userdata(data) => {
                let data = data.downcast_ref::<T>().unwrap();
                RLock::new(data.clone())
            },
            _ => panic!("Got {:?}", value),
        }
    }
}

impl<'vm, T> Pushable<'vm> for RLock<T> where T: Userdata + Clone {
    fn push(self, thread: &'vm Thread, context: &mut Context) -> Result<(), vm::Error> {
        let data: Box<Userdata> = Box::new(self.read().unwrap().clone());
        let userdata = context.alloc_with(thread, Move(data))?;
        context.stack.push(Value::Userdata(userdata));
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Entity(specs::Entity);

impl Deref for Entity {
    type Target = specs::Entity;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

register_gluon!(Entity);

register_gluon!(InitialPosition);
register_data!(InitialPosition);

#[derive(Component, Clone, Debug)]
pub struct Data(Arc<Mutex<AnyMap<any::CloneAny + Send + Sync>>>);

register_gluon!(Data);

impl Data {
    pub fn new() -> Self {
        Data(Arc::new(Mutex::new(AnyMap::new())))
    }

    pub fn insert<T: 'static>(&self, data: T) -> Option<T> where T: Clone + Send + Sync {
        let mut map = self.0.lock().unwrap();
        map.insert(data)
    }

    pub fn insert_flip<T: 'static>(data: T, map: Self) -> Option<T> where T: Clone + Send + Sync  {
        let mut map = map.0.lock().unwrap();
        map.insert(data)
    }

    pub fn get<T: 'static>(&self) -> Option<T> where T: Clone + Send + Sync  {
        let mut map = self.0.lock().unwrap();
        map.get().map(|t: &T| t.clone())
    }

    pub fn remove<T: 'static>(&self) -> Option<T> where T: Clone + Send + Sync  {
        let mut map = self.0.lock().unwrap();
        map.remove()
    }

    pub fn contains<T: 'static>(&self) -> bool where T: Clone + Send + Sync  {
        let mut map = self.0.lock().unwrap();
        map.contains::<T>()
    }
}

#[derive(Component, Copy, Clone, Debug)]
pub struct RequireMap;

#[derive(Component, Clone, Debug)]
pub struct DataReference {
    pub entity: Option<specs::Entity>,
    pub map: Option<Data>,
}

impl DataReference {
    pub fn entity(&self) -> Option<Entity> {
        self.entity.map(|e| Entity(e))
    }

    pub fn data(&self) -> Option<Data> {
        self.map.clone().map(|m| m.clone())
    }
}

register_gluon!(DataReference);
register_data!(DataReference);

pub fn register_opalite_api(vm: &gluon::Thread) {
    cgmath::register_opalite_api(vm);
    conrod::register_opalite_api(vm);

    vm.register_type::<Data>("Data", &[]).unwrap();
    vm.register_type::<DataReference>("DataReference", &[]).unwrap();
    vm.register_type::<Entity>("Entity", &[]).unwrap();
    vm.register_type::<Map>("Map", &[]).unwrap();

    vm.register_type::<InitialPosition>("InitialPosition", &[]).unwrap();

    gluon::import::add_extern_module(vm, "initial_position", |vm: &gluon::Thread| {
        vm::ExternModule::new(vm, record!(
            x => primitive!(1 InitialPosition::x),
            y => primitive!(1 InitialPosition::y),
            z => primitive!(1 InitialPosition::z),
            data => record!(
                insert => primitive!(2 InitialPosition::insert_to_data),
                get => primitive!(1 InitialPosition::get_from_data),
                remove => primitive!(1 InitialPosition::remove_from_data),
                contains => primitive!(1 InitialPosition::contains_in_data),
            ),
        ))
    });

    gluon::import::add_extern_module(vm, "data_reference", |vm: &gluon::Thread| {
        vm::ExternModule::new(vm, record!(
            entity => primitive!(1 DataReference::entity),
            data => primitive!(1 DataReference::data),
            insert => primitive!(2 DataReference::insert_to_data),
            get => primitive!(1 DataReference::get_from_data),
            remove => primitive!(1 DataReference::remove_from_data),
            contains => primitive!(1 DataReference::contains_in_data),
        ))
    });

    fn map_location(map: &Map, entity: &Entity) -> Option<self::cgmath::Vec3> {
        map.location(&entity.0).map(|l| {
            self::cgmath::Vec3(::cgmath::Vector3::new(l.x as f64, l.y as f64, l.z as f64))
        })
    }

    gluon::import::add_extern_module(vm, "map", |vm: &gluon::Thread| {
        vm::ExternModule::new(vm, record!(
            location => primitive!(2 map_location),
            data => record!(
                insert => primitive!(2 RLock::<Map>::insert_to_data),
                get => primitive!(1 RLock::<Map>::get_from_data),
                remove => primitive!(1 RLock::<Map>::remove_from_data),
                contains => primitive!(1 RLock::<Map>::contains_in_data),
            ),
        ))
    });
}

pub struct RequireMapSystem;

impl RequireMapSystem {
    pub fn new() -> Self {
        RequireMapSystem
    }
}

impl<'a> System<'a> for RequireMapSystem {
    type SystemData =  (Entities<'a>,
                        ReadStorage<'a, RequireMap>,
                        ReadStorage<'a, Data>,
                        Fetch<'a, RLock<Map>>);

    fn run(&mut self, (entities, require_maps, datas, map): Self::SystemData) {
        use specs::Join;

        for (_, data) in (&require_maps, &datas).join() {
            data.insert(map.clone());
        }
    }
}

pub struct DataReferenceSystem;

impl DataReferenceSystem {
    pub fn new() -> Self {
        DataReferenceSystem
    }
}

impl<'a> System<'a> for DataReferenceSystem {
    type SystemData =  (Entities<'a>,
                        WriteStorage<'a, DataReference>,
                        ReadStorage<'a, Data>);

    fn run(&mut self, (entities, mut data_refs, datas): Self::SystemData) {
        use specs::Join;

        for (data_ref, data) in (&mut data_refs, &datas).join() {
            if let Some(entity) = data_ref.entity {
                if let Some(map) = datas.get(entity) {
                    data_ref.map = Some(map.clone());
                    data.insert(data_ref.clone());
                } else {
                    data_ref.map = None;
                    data.remove::<DataReference>();
                }
            }
        }
    }
}

#[derive(Component)]
#[component(VecStorage)]
pub struct GluonUiComponent {
    pub name: String,
    pub expr: String,
}

pub struct GluonUiSystem;

impl GluonUiSystem {
    pub fn new() -> Self {
        GluonUiSystem
    }
}

impl<'a> System<'a> for GluonUiSystem {
    type SystemData =  (ReadStorage<'a, GluonUiComponent>,
                        ReadStorage<'a, Data>,
                        FetchMut<'a, GluonUi>,
                        FetchMut<'a, Gluon>);

    fn run(&mut self, (ui_component, data, mut ui, mut gluon): Self::SystemData) {
        use specs::Join;

        let Gluon { compiler, thread } = &mut *gluon;

        for (ui_component, data) in (&ui_component, &data).join() {
            let mut expr = prelude::add_prelude::<Data, conrod::GluonWidget>(&vec!["data"][..], &ui_component.name, &ui_component.expr, compiler, thread.clone());
            let res = match expr {
                Ok(mut expr) => match expr.call(data.clone()) {
                    Ok(expr) => expr,
                    Err(err) => {
                        println!("Gluon Error: {}", err);
                        continue
                    },
                },
                Err(err) => {
                    println!("Gluon Error: {}", err);
                    continue
                },
            };

            ui.insert(res.name().to_owned(), res.clone());
        }
    }
}
