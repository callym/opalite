#[macro_export] macro_rules! register_gluon {
    ($gluon_ty:ident) => (
        impl Userdata for $gluon_ty { }

        impl VmType for $gluon_ty {
            type Type = $gluon_ty;
        }

        impl Traverseable for $gluon_ty {
            fn traverse(&self, _: &mut Gc) { }
        }

        impl<'vm> Getable<'vm> for $gluon_ty {
            fn from_value(vm: &'vm Thread, value: Variants) -> Self {
                match value.as_ref() {
                    ValueRef::Userdata(data) => {
                        let data = data.downcast_ref::<Self>().unwrap();
                        data.clone()
                    },
                    _ => panic!("Got {:?}", value),
                }
            }
        }

//        impl<'vm> Pushable<'vm> for $gluon_ty { }
    )
}

#[macro_export] macro_rules! register_data {
    ($data_ty:ident) => (
        impl $data_ty {
            pub fn get_from_data(map: Data) -> Option<$data_ty> {
                map.get()
            }

            pub fn insert_to_data(map: Data, data: $data_ty) -> Option<$data_ty> {
                map.insert(data)
            }

            pub fn remove_from_data(map: Data) -> Option<$data_ty> {
                map.remove()
            }

            pub fn contains_in_data(map: Data) -> bool {
                map.contains::<$data_ty>()
            }
        }
    );
}
