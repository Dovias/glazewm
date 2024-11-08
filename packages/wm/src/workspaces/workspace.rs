use std::{
  cell::{Ref, RefCell, RefMut},
  collections::VecDeque,
  rc::Rc,
};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
  common::{Rect, TilingDirection},
  containers::{
    traits::{CommonGetters, PositionGetters, TilingDirectionGetters},
    Container, ContainerDto, DirectionContainer, TilingContainer,
    WindowContainer
  },
  impl_common_getters, impl_container_debug,
  impl_tiling_direction_getters,
  user_config::{GapsConfig, WorkspaceConfig}, windows::{traits::WindowGetters, WindowState}
};

#[derive(Clone)]
pub struct Workspace(Rc<RefCell<WorkspaceInner>>);

#[derive(Debug)]
struct WorkspaceInner {
  id: Uuid,
  parent: Option<Container>,
  children: VecDeque<Container>,
  child_focus_order: VecDeque<Uuid>,
  config: WorkspaceConfig,
  gaps_config: GapsConfig,
  tiling_direction: TilingDirection,
}

/// User-friendly representation of a workspace.
///
/// Used for IPC and debug logging.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDto {
  id: Uuid,
  name: String,
  display_name: Option<String>,
  parent_id: Option<Uuid>,
  children: Vec<ContainerDto>,
  child_focus_order: Vec<Uuid>,
  has_focus: bool,
  is_displayed: bool,
  width: i32,
  height: i32,
  x: i32,
  y: i32,
  tiling_direction: TilingDirection,
}

impl Workspace {
  pub fn new(
    config: WorkspaceConfig,
    gaps_config: GapsConfig,
    tiling_direction: TilingDirection,
  ) -> Self {
    let workspace = WorkspaceInner {
      id: Uuid::new_v4(),
      parent: None,
      children: VecDeque::new(),
      child_focus_order: VecDeque::new(),
      config,
      gaps_config,
      tiling_direction,
    };

    Self(Rc::new(RefCell::new(workspace)))
  }

  pub fn get_fullscreen_window(&self) -> Option<WindowContainer> {
    match self.borrow_children().iter().find(|container| {
      if let Ok(window_container) = container.as_window_container() {
        matches!(window_container.state(), WindowState::Fullscreen(_))
      } else {
        false 
      } 
    }) {
      Some(container) => Some(container.as_window_container().ok()?),
      _ => None
    }
  }

  /// Underlying config for the workspace.
  pub fn config(&self) -> WorkspaceConfig {
    self.0.borrow().config.clone()
  }

  /// Update the underlying config for the workspace.
  pub fn set_config(&self, config: WorkspaceConfig) {
    self.0.borrow_mut().config = config;
  }

  /// Whether the workspace is currently displayed by the parent monitor.
  pub fn is_displayed(&self) -> bool {
    self
      .monitor()
      .and_then(|monitor| monitor.displayed_workspace())
      .map(|workspace| workspace.id() == self.id())
      .unwrap_or(false)
  }

  pub fn set_gaps_config(&self, gaps_config: GapsConfig) {
    self.0.borrow_mut().gaps_config = gaps_config;
  }

  pub fn to_dto(&self) -> anyhow::Result<ContainerDto> {
    let rect = self.to_rect()?;
    let config = self.config();

    let children = self
      .children()
      .iter()
      .map(|child| child.to_dto())
      .try_collect()?;

    Ok(ContainerDto::Workspace(WorkspaceDto {
      id: self.id(),
      name: config.name,
      display_name: config.display_name,
      parent_id: self.parent().map(|parent| parent.id()),
      children,
      child_focus_order: self.0.borrow().child_focus_order.clone().into(),
      has_focus: self.has_focus(None),
      is_displayed: self.is_displayed(),
      width: rect.width(),
      height: rect.height(),
      x: rect.x(),
      y: rect.y(),
      tiling_direction: self.tiling_direction(),
    }))
  }
}

impl_container_debug!(Workspace);
impl_common_getters!(Workspace);
impl_tiling_direction_getters!(Workspace);

impl PositionGetters for Workspace {
  fn to_rect(&self) -> anyhow::Result<Rect> {
    let monitor = self.monitor()
      .context("Workspace has no parent monitor.")?;

    let monitor_rect = monitor.to_rect()?;
    let native_monitor = monitor.native();

    // Get delta between monitor bounds and its working area.
    let working_area_delta = native_monitor
      .working_rect()
      .context("Failed to get working area of parent monitor.")?
      .delta(&monitor_rect);

    let gaps_config = &self.0.borrow().gaps_config;
    let outer_gap_delta = &gaps_config.outer_gap;
    let scale_factor = Some(match &gaps_config.scale_with_dpi {
      true => native_monitor.scale_factor()?,
      false => 1.,
    });
    
    let monitor_width = monitor_rect.width();
    let monitor_height = monitor_rect.height();
    Ok(
      Rect::from_ltrb(
      monitor_rect.left
          + working_area_delta.left.to_px(monitor_width, scale_factor)
          + outer_gap_delta.left.to_px(monitor_width, scale_factor),
      monitor_rect.top
          + working_area_delta.top.to_px(monitor_height, scale_factor)
          + outer_gap_delta.top.to_px(monitor_height, scale_factor),
      monitor_rect.right
          + working_area_delta.right.to_px(monitor_width, scale_factor)
          - outer_gap_delta.right.to_px(monitor_width, scale_factor),
      monitor_rect.bottom
          + working_area_delta.bottom.to_px(monitor_height, scale_factor)
          - outer_gap_delta.bottom.to_px(monitor_height, scale_factor),
      )
    )
  }
}
