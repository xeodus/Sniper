use barter::engine::state::{global::DefaultGlobalData, instrument::data::DefaultInstrumentMarketData, EngineState};
use barter_execution::order::request::{OrderRequestCancel, OrderRequestOpen};
pub struct BarterStrategy { pub inner: crate::strategy::MarketData }
impl barter::strategy::algo::AlgoStrategy for BarterStrategy {
    type State = EngineState<DefaultGlobalData, DefaultInstrumentMarketData>;

    fn generate_algo_orders(
            &self,
            state: &Self::State,
        ) -> (
            impl IntoIterator<Item = OrderRequestCancel>,
            impl IntoIterator<Item = OrderRequestOpen>,
        ) {
        let _ = state;
        (vec![], vec![])
    }


}
