use cgmath::Vector3;

pub fn float(v: f32) -> ::ordered_float::NotNaN<f32> {
    ::ordered_float::NotNaN::new(v).unwrap()
}

pub fn vec3(v0: f32, v1: f32, v2: f32) -> [::ordered_float::NotNaN<f32>; 3] {
    [float(v0), float(v1), float(v2)]
}

pub fn from_vec3(vec: Vector3<f32>) -> Vector3<::ordered_float::NotNaN<f32>> {
    let vec = vec3(vec.x, vec.y, vec.z);
    Vector3::new(vec[0], vec[1], vec[2])
}

pub fn vec4(v0: f32, v1: f32, v2: f32, v3: f32) -> [::ordered_float::NotNaN<f32>; 4] {
    [float(v0), float(v1), float(v2), float(v3)]
}
