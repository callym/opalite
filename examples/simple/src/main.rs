#![feature(nll)]

extern crate opalite;

use opalite::{
    Component,
    ComponentStores,
    Future,
    FutureExt,
    GameLoop,
    Handler,
    HashStore,
    Id,
    MoveMessage,
    MapStore,
    Store,
};

struct Foo {
    message: &'static str,
}

impl Component for Foo { }

fn main() {
    let mut game_loop = GameLoop::new();

    let x = 50;
    let y = 50;
    let z = 50;

    let mut stores = ComponentStores::new_with_default_components(x, y, z);

    let mut foo_store = HashStore::new();
    let foo_boo = Foo { message: "boo!" };
    let foo_boo_id = Id::new();
    let foo_bar = Foo { message: "bar!" };
    let foo_bar_id = Id::new();
    foo_store.set(foo_bar_id, foo_bar);
    foo_store.set(foo_boo_id, foo_boo);

    stores.register(foo_store);

    stores.do_with::<MapStore, _, _>(|map_store | {
        map_store.do_tile_at((10, 10, 10), |tile| tile.entities.insert(foo_bar_id));
        map_store.do_tile_at((15, 15, 10), |tile| tile.entities.insert(foo_boo_id));
    });

    for tile in stores.iter::<MapStore>() {
        for id in &tile.entities {
            println!("{:?} - {:?}", tile.coordinates(), stores.do_with_component::<HashStore<Foo>, _, _>(id, |f| f.message));
        }
    }

    stores.do_with::<MapStore, _, _>(|map_store| {
        let message = MoveMessage {
            id: foo_bar_id,
            coordinates: (11, 11, 10),
        };

        let future = map_store.send(message);

        let future = future
            .map(|_| println!("MOVED!"))
            .map_err(|_| println!("FAILED TO MOVE"));

        game_loop.spawn(future);
    });

    for tile in stores.iter::<MapStore>() {
        for id in &tile.entities {
            println!("{:?} - {:?}", tile.coordinates(), stores.do_with_component::<HashStore<Foo>, _, _>(id, |f| f.message));
        }
    }

    println!("Hello, world!");

    game_loop.run();
}
