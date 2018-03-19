pub trait PushConstant {
    const SIZE: u32;

    fn data(&self) -> Vec<u32>;
}
