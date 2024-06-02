/// A trait for objects that can be processable. I.e. objects that need regular updates
pub trait Processable {
    /// function called regularly to update object's state
    ///
    /// `delta`: elapsed time between this call and the last call.
    /// The caller of this method is responsible of providing this value
    fn process(&mut self, _delta: f64) {}
}
