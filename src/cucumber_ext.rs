//! Extensions to Cucumber for better integration with external systems

use crate::{Cucumber, World, Parser, Writer};
use crate::runner::Basic;

impl<W, P, I, Wr, Cli> Cucumber<W, P, I, Basic<W>, Wr, Cli>
where
    W: World,
    P: Parser<I>,
    Wr: Writer<W>,
    Cli: clap::Args,
{
    /// Register an observer for test execution monitoring
    /// 
    /// This allows external systems like ObservaBDD to observe test execution
    /// without modifying the writer chain.
    /// 
    /// # Example
    /// ```rust
    /// # use cucumber::{Cucumber, World};
    /// # #[derive(Debug, Default, World)]
    /// # struct TestWorld;
    /// # #[cfg(feature = "observability")]
    /// # async fn example() {
    /// let cucumber = TestWorld::cucumber()
    ///     .register_observer(Box::new(my_observer));
    /// # }
    /// ```
    #[cfg(feature = "observability")]
    pub fn register_observer(
        mut self,
        observer: Box<dyn crate::observer::TestObserver<W>>,
    ) -> Self {
        self.runner = self.runner.register_observer(observer);
        self
    }
}