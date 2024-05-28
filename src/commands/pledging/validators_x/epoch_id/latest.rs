#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::validators_x::network_view_at_block::NetworkViewAtBlockArgsContext)]
#[interactive_clap(output_context = LatestContext)]
pub struct Latest {}

#[derive(Debug, Clone)]
pub struct LatestContext;

impl LatestContext {
    pub fn from_previous_context(
        previous_context: super::super::network_view_at_block::NetworkViewAtBlockArgsContext,
        _scope: &<Latest as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        super::super::block_id::display_current_validators_info(
            unc_primitives::types::EpochReference::Latest,
            &previous_context.network_config,
        )?;
        Ok(Self)
    }
}
