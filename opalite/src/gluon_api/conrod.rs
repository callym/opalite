use conrod::{ self, Borderable, Colorable, Positionable, widget };
use gluon::{
    self,
    vm::{
        self,
        api::{ Getable, Userdata, ValueRef, VmType },
        gc::{ Gc, Traverseable },
        Variants,
    },
    Thread,
};

macro_rules! widget_impl {
    ($widget:ident) => (
        #[derive(Debug, Clone)]
        pub struct $widget(pub(crate) widget::$widget, pub(crate) String);
    )
}

macro_rules! widget_impl_ext {
    ($widget:ident) => (
        impl $widget {
            pub fn name(&self) -> &str {
                &self.1
            }

            fn build(widget: Self) -> GluonWidget {
                widget.into()
            }
        }

        impl Into<GluonWidget> for $widget {
            fn into(self) -> GluonWidget {
                GluonWidget::$widget { value: self }
            }
        }
    )
}

macro_rules! colorable_impl {
    ($widget:ident) => (
        impl $widget {
            fn color(color: Color, widget: Self) -> Self {
                let $widget(widget, name) = widget;
                let widget = widget.color(color.0);
                $widget(widget, name)
            }
        }
    )
}

macro_rules! positionable_impl {
    ($widget:ident) => (
        impl $widget {
            fn x(x: f64, widget: Self) -> Self {
                let $widget(widget, name) = widget;
                let widget = widget.x(x);
                $widget(widget, name)
            }

            fn y(y: f64, widget: Self) -> Self {
                let $widget(widget, name) = widget;
                let widget = widget.y(y);
                $widget(widget, name)
            }

            fn x_y(x: f64, y: f64, widget: Self) -> Self {
                let $widget(widget, name) = widget;
                let widget = widget.x_y(x, y);
                $widget(widget, name)
            }
        }
    )
}

macro_rules! borderable_impl {
    ($widget:ident) => (
        impl $widget {
            fn border(width: f64, widget: Self) -> Self {
                let $widget(widget, name) = widget;
                let widget = widget.border(width);
                $widget(widget, name)
            }

            fn border_color(color: Color, widget: Self) -> Self {
                let $widget(widget, name) = widget;
                let widget = widget.border_color(color.0);
                $widget(widget, name)
            }
        }
    )
}

macro_rules! module {
    (
        register: [ $($register_ty:ident),* $(,)? ],
        borderable: [
            $(
                [$widget_b:ident: $mod_name_b:ident] -> {
                    $($fun_b:ident: $num_b:tt),*
                    $(,)?
                } $(,)?
            )*
        ],
        colorable: [
            $(
                [$widget_c:ident: $mod_name_c:ident] -> {
                    $($fun_c:ident: $num_c:tt),*
                    $(,)?
                } $(,)?
            )*
        ],
        other: [
            $(
                [$widget:ident: $mod_name:ident] -> {
                    $($fun:ident: $num:tt),*
                    $(,)?
                } $(,)?
            )*
        ],
        $(,)?
        $($fun_alone:ident: $num_alone:tt),*
        $(,)?
    ) => {
        #[derive(Debug, Clone)]
        pub enum GluonWidget {
            $( $widget_b { value: $widget_b }, )*
            $( $widget_c { value: $widget_c }, )*
            $( $widget { value: $widget }, )*
        }

        impl GluonWidget {
            pub fn name(&self) -> &str {
                match self {
                    $( GluonWidget::$widget_b { value } => value.name(), )*
                    $( GluonWidget::$widget_c { value } => value.name(), )*
                    $( GluonWidget::$widget { value } => value.name(), )*
                }
            }
        }

        register_gluon!(GluonWidget);

        $(
            register_gluon!($register_ty);
        )*

        $(
            register_gluon!($widget_b);
            widget_impl!($widget_b);
            widget_impl_ext!($widget_b);
            positionable_impl!($widget_b);
            borderable_impl!($widget_b);
            colorable_impl!($widget_b);
        )*

        $(
            register_gluon!($widget_c);
            widget_impl!($widget_c);
            widget_impl_ext!($widget_c);
            positionable_impl!($widget_c);
            colorable_impl!($widget_c);
        )*

        $(
            register_gluon!($widget);
            widget_impl_ext!($widget);
            positionable_impl!($widget);
        )*

        pub fn register_opalite_api(vm: &gluon::Thread) {
            $(
                vm.register_type::<$register_ty>(stringify!($register_ty), &[]).unwrap();
            )*

            $(
                vm.register_type::<$widget_b>(stringify!($widget_b), &[]).unwrap();
            )*
            $(
                vm.register_type::<$widget_c>(stringify!($widget_c), &[]).unwrap();
            )*
            $(
                vm.register_type::<$widget>(stringify!($widget), &[]).unwrap();
            )*
            vm.register_type::<GluonWidget>("GluonWidget", &[]).unwrap();

            gluon::import::add_extern_module(vm, "conrod", |vm: &gluon::Thread| {
                vm::ExternModule::new(vm, record!(
                    $($mod_name_b => record!(
                        $(
                            $fun_b => primitive!($num_b $widget_b::$fun_b),
                        )*
                        x => primitive!(2 $widget_b::x),
                        y => primitive!(2 $widget_b::y),
                        x_y => primitive!(3 $widget_b::x_y),
                        border => primitive!(2 $widget_b::border),
                        border_color => primitive!(2 $widget_b::border_color),
                        color => primitive!(2 $widget_b::color),
                        build => primitive!(1 $widget_b::build)
                    ),)*
                    $($mod_name_c => record!(
                        $(
                            $fun_c => primitive!($num_c $widget_c::$fun_c),
                        )*
                        x => primitive!(2 $widget_c::x),
                        y => primitive!(2 $widget_c::y),
                        x_y => primitive!(3 $widget_c::x_y),
                        color => primitive!(2 $widget_c::color),
                        build => primitive!(1 $widget_c::build)
                    ),)*
                    $($mod_name => record!(
                        $(
                            $fun => primitive!($num $widget::$fun),
                        )*
                        x => primitive!(2 $widget::x),
                        y => primitive!(2 $widget::y),
                        x_y => primitive!(3 $widget::x_y),
                        build => primitive!(1 $widget::build)
                    ),)*
                    $($fun_alone => primitive!($num_alone $fun_alone))*,
                ))
            });
        }
    };
}

#[derive(Copy, Clone, Debug)]
pub struct Color(pub(crate) conrod::Color);

fn rgba(r: f64, g: f64, b: f64, a: f64) -> Color {
    Color(conrod::Color::Rgba(r as f32, g as f32, b as f32, a as f32))
}

#[derive(Debug, Clone)]
pub struct Oval(pub(crate) widget::Oval<widget::oval::Full>, pub(crate) String);

module!(
    register: [ Color ],
    borderable: [
        [BorderedRectangle: bordered_rectangle] -> {
            new: 3,
        },
    ],
    colorable: [
        [Rectangle: rectangle] -> {
            new: 3,
        },
    ],
    other: [
        [Oval: oval] -> {
            new: 3,
        }
    ],
    rgba: 4,
);

impl BorderedRectangle {
    fn new(x: f64, y: f64, name: String) -> Self {
        BorderedRectangle(widget::BorderedRectangle::new([x, y]), name)
    }
}

impl Rectangle {
    fn new(x: f64, y: f64, name: String) -> Self {
        Rectangle(widget::Rectangle::fill([x, y]), name)
    }
}

impl Oval {
    fn new(x: f64, y: f64, name: String) -> Self {
        Oval(widget::Oval::fill([x, y]), name)
    }
}
