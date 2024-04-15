use crate::containers::{
  traits::{CommonGetters, TilingSizeGetters},
  TilingContainer,
};

const MIN_TILING_SIZE: f32 = 0.01;

pub fn resize_tiling_container(
  container_to_resize: &TilingContainer,
  target_size: f32,
) {
  let tiling_siblings =
    container_to_resize.tiling_siblings().collect::<Vec<_>>();

  // Ignore cases where the container is the only child.
  if tiling_siblings.is_empty() {
    return;
  }

  // Get available tiling size amongst siblings.
  let available_size = available_size(&tiling_siblings);

  // Prevent the container from being smaller than the minimum size, and
  // larger than the space available from sibling containers.
  let clamped_target_size = target_size
    .max(MIN_TILING_SIZE - container_to_resize.tiling_size())
    .min(available_size);

  // Resize the container.
  container_to_resize.set_tiling_size(
    container_to_resize.tiling_size() + clamped_target_size,
  );

  // Distribute the available tiling size amongst its siblings.
  for sibling in &tiling_siblings {
    let sibling_size_delta = sibling_size_delta(
      &sibling,
      tiling_siblings.len(),
      clamped_target_size,
      available_size,
    );

    sibling.set_tiling_size(sibling.tiling_size() - sibling_size_delta);
  }
}

fn available_size(containers: &Vec<TilingContainer>) -> f32 {
  containers.iter().fold(0.0, |sum, container| {
    sum + container.tiling_size() - MIN_TILING_SIZE
  })
}

fn sibling_size_delta(
  sibling_to_resize: &TilingContainer,
  sibling_count: usize,
  target_size: f32,
  available_size: f32,
) -> f32 {
  let con_available_size =
    sibling_to_resize.tiling_size() - MIN_TILING_SIZE;

  // Get percentage of resize that affects this container. The available
  // tiling size here can be 0 when the main container to resize is shrunk
  // from the max tiling size.
  let resize_factor = if available_size == 0.0 || target_size < 0.0 {
    1.0 / sibling_count as f32
  } else {
    con_available_size / available_size
  };

  resize_factor * target_size
}
