use core::marker::PhantomData;

pub enum State {
    NotInit,
    Init,
}

pub struct BMI270<T, S> {
    interface: T,
    state: PhantomData<S>,
}

impl<T, S> BMI270<T, S> {
    //
}
