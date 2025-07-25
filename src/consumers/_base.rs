pub trait Consumer {
    fn start(&self);
    fn stop(&self);
}
