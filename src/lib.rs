
pub extern crate corona;
pub extern crate generator;

#[macro_use]
extern crate log;
extern crate futures;
extern crate tokio;
extern crate failure;
extern crate through;

use self::error::*;

mod error;

use std::time::{Duration, Instant};
use std::fmt::Display;
use std::borrow::Cow;

use corona::prelude::CoroutineFuture;
use futures::prelude::*;
use corona::{
    Coroutine as Corona,
    CoroutineResult as CoronaHandle,
    errors::TaskFailed as CoronaError,
};
use tokio::runtime::current_thread;
use tokio::io::Error as TokioError;
use tokio::timer::Delay;
use through::*;

pub mod prelude {
    pub use crate::{
        routine,
        JokerFutureExt,
        joker_sleep,
        joker_sleep_until,
    };
    pub use crate::error::*;
    pub use corona::prelude::{
        CoroutineFuture,
        CoroutineSink,
        CoroutineStream,
    };
    pub use generator::Gn as Generator;
}

pub fn routine<I, E, F>(func: F) -> JokerRoutine<I, E, F>
    where I: 'static,
          E: 'static,
          F: FnOnce() -> Result<I, E> + Send + Sync + 'static,
          JokerError: From<E>{
    JokerRoutine {
        state: JokerRoutineState::Func(func)
    }
}

pub struct JokerRoutine<I, E, F>
    where I: 'static,
          E: 'static,
          F: FnOnce() -> Result<I, E> + Send + Sync + 'static,
          JokerError: From<E>{
    state: JokerRoutineState<I, E, F>,
}

enum JokerRoutineState<I, E, F>
    where I: 'static,
          E: 'static,
          F: FnOnce() -> Result<I, E> + Send + Sync + 'static,
          JokerError: From<E>{
    Func(F),
    Handle(CoronaHandle<Result<I, E>>),
    Depleted,
}

impl<I, E, F> JokerRoutineState<I, E, F>
    where I: 'static,
          E: 'static,
          F: FnOnce() -> Result<I, E> + Send + Sync + 'static,
          JokerError: From<E>{
    fn transform(self) -> (Self, Result<Async<I>, JokerError>) {
        match self {
            JokerRoutineState::Func(func) => {
                JokerRoutineState::Handle(Corona::with_defaults(func))
                    .transform()
            },
            JokerRoutineState::Handle(mut handle) => match handle.poll() {
                Ok(Async::Ready(result)) => (
                    JokerRoutineState::Depleted,
                    match result {
                        Ok(item) => Ok(Async::Ready(item)),
                        Err(e) => Err(JokerError::from(e)),
                    }
                ),
                Err(e) => (
                    JokerRoutineState::Depleted,
                    Err(From::<CoronaError>::from(e)),
                ),
                Ok(Async::NotReady) => (
                    JokerRoutineState::Handle(handle),
                    Ok(Async::NotReady),
                ),
            },
            JokerRoutineState::Depleted => panic!("polling of depleted future"),
        }
    }
}

impl<I, E, F> Future for JokerRoutine<I, E, F>
    where I: 'static,
          E: 'static,
          F: FnOnce() -> Result<I, E> + Send + Sync + 'static,
          JokerError: From<E>{
    type Item = I;
    type Error = JokerError;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        through_and(&mut self.state, JokerRoutineState::transform)
    }
}

pub trait JokerFutureExt: Sized + Future {
    fn tokio_bootstrap(self) -> Result<Self::Item, Self::Error>
        where Self::Item: Send + 'static,
              Self::Error: Send + 'static,
              Self::Error: From<TokioError>;

    fn tokio_spawn(self)
        where Self: Send + 'static,
              (): From<Self::Item>,
              (): From<Self::Error>;

    fn log_err<N>(self, name: N) -> LogError<Self>
        where Self::Error: Display,
              Cow<'static, str>: From<N>;
}

impl<F: Future> JokerFutureExt for F {
    fn tokio_bootstrap(self) -> Result<Self::Item, Self::Error>
        where Self::Item: Send + 'static,
              Self::Error: Send + 'static,
              Self::Error: From<TokioError> {

        current_thread::block_on_all(self)
    }

    fn tokio_spawn(self)
        where Self: Send + 'static,
              (): From<Self::Item>,
              (): From<Self::Error> {
        tokio::spawn(self
            .map(From::from)
            .map_err(From::from));
    }

    fn log_err<N>(self, name: N) -> LogError<Self>
        where Self::Error: Display,
              Cow<'static, str>: From<N> {
        LogError {
            inner: self,
            name: Cow::from(name),
        }
    }
}

pub struct LogError<F>
    where F: Future,
          F::Error: Display {
    inner: F,
    name: Cow<'static, str>,
}

impl<F> Future for LogError<F>
    where F: Future,
          F::Error: Display {
    type Item = F::Item;
    type Error = ();

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        self.inner.poll()
            .map_err(|e| error!("({}) {}", self.name, e))
    }
}

pub fn joker_sleep_until(instant: Instant) -> Result<(), tokio::timer::Error> {
    Delay::new(instant)
        .coro_wait()
}

pub fn joker_sleep(duration: Duration) -> Result<(), tokio::timer::Error> {
    joker_sleep_until(Instant::now() + duration)
}

#[test]
fn test() {
    extern crate pretty_env_logger;

    use failure::Error;

    pretty_env_logger::init();

    fn countdown() -> Result<(), Error> {
        for i in (1..=5).rev() {
            println!("in {}...", i);
            joker_sleep(Duration::from_secs(1))
                .map_err(Error::from)?;
        }

        println!("shazam!");

        Ok(())
    }

    routine(|| -> Result<(), Infallible> {
        println!("hoo hoo");

        routine(countdown)
            .log_err("countdown")
            .tokio_spawn();

        Ok(())
    })
        .tokio_bootstrap()
        .unwrap();

    println!("yay");
}