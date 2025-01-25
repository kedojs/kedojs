
// Custom TryClone trait
pub trait TryClone: Sized {
    fn try_clone(&self) -> Option<Self>;
}