use {HandleRequest, HandlerOptions, Result};

#[derive(Debug)]
pub struct Dispatcher {}

#[derive(Debug)]
pub struct DispatcherBuilder {}
impl DispatcherBuilder {
    pub fn new() -> Self {
        DispatcherBuilder {}
    }

    pub fn register_handler<H, D, E>(
        &mut self,
        handler: H,
        options: HandlerOptions<H, D, E>,
    ) -> Result<()>
    where
        H: HandleRequest,
    {
        unimplemented!()
    }

    pub fn finish(self) -> Dispatcher {
        unimplemented!()
    }
}
