use super::{GenericInstancePayload, send_to_plugin};

use crate::shared::{ActionContext, ActionInstance};

#[derive(serde::Serialize)]
struct AppearEvent {
	event: &'static str,
	action: String,
	context: ActionContext,
	device: String,
	payload: GenericInstancePayload,
}

pub async fn will_appear(instance: &ActionInstance) -> Result<(), anyhow::Error> {
	send_to_plugin(
		&instance.action.plugin,
		&AppearEvent {
			event: "willAppear",
			action: instance.action.uuid.clone(),
			context: instance.context.clone(),
			device: instance.context.device.clone(),
			payload: GenericInstancePayload::new(instance),
		},
	)
	.await?;

	super::states::title_parameters_did_change(instance, instance.current_state).await?;

	Ok(())
}

pub async fn will_disappear(instance: &ActionInstance, clear_on_device: bool) -> Result<(), anyhow::Error> {
	send_to_plugin(
		&instance.action.plugin,
		&AppearEvent {
			event: "willDisappear",
			action: instance.action.uuid.clone(),
			context: instance.context.clone(),
			device: instance.context.device.clone(),
			payload: GenericInstancePayload::new(instance),
		},
	)
	.await?;

	if clear_on_device {
		let mut context: crate::shared::Context = (&instance.context).into();
		if context.controller == "Keypad" {
			if let Some(device_info) = crate::shared::DEVICES.get(&context.device) {
				let page_size = (device_info.rows * device_info.columns) as usize;
				if page_size > 0 {
					context.position = (context.position as usize % page_size) as u8;
				}
			}
		}
		if let Err(error) = crate::events::outbound::devices::update_image(context, None).await {
			log::warn!("Failed to clear device image: {}", error);
		}
	}

	Ok(())
}
