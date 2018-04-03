pub fn float(v: f32) -> ::ordered_float::NotNaN<f32> {
    ::ordered_float::NotNaN::new(v).unwrap()
}

pub fn vec4(v0: f32, v1: f32, v2: f32, v3: f32) -> [::ordered_float::NotNaN<f32>; 4] {
    [float(v0), float(v1), float(v2), float(v3)]
}
