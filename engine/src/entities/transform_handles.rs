use glam::{Quat, Vec3, Vec3Swizzles};
use rapier3d::{
    geometry::{
        ColliderBuilder, ColliderHandle, Group, InteractionGroups, InteractionTestMode, Ray,
    },
    math::Pose,
    pipeline::QueryFilter,
};
use thunderdome::Index;

use crate::{
    components::TransformComponent,
    game::{CONE_MESH_ID, GameContext, SPHERE_MESH_ID},
    input::InputHandler,
    physics::{
        HANDLE_INTERACTION_GROUP, UserdataType, context::PhysicsContext, gen_userdata,
        read_userdata,
    },
    renderer::render_scene::{MeshNode, RenderScene},
};

const HANDLE_OFFSET: f32 = 5.0;
const HANDLE_SIZE: f32 = 5.0;
const HANDLE_OPACITY: f32 = 0.5;
const HANDLE_OPACITY_HOVERING: f32 = 1.0;

const HANDLE_AXES: &'static [Vec3] = &[
    Vec3::X,
    Vec3::Y,
    Vec3::NEG_Z,
    Vec3::NEG_X,
    Vec3::NEG_Y,
    Vec3::Z,
];

const HANDLE_AXES_COLOR: &'static [Vec3] = &[Vec3::X, Vec3::Y, Vec3::Z, Vec3::X, Vec3::Y, Vec3::Z];

const DRAGGER_SENSITIVITY: f32 = 2.0;

#[derive(Debug, PartialEq, Eq)]
pub enum TransformType {
    Position,
    Rotation,
    Size,
}

struct TransformHandle {
    transform: TransformComponent,
    collision_box: ColliderHandle,
    axis: Vec3,
    color: Vec3,
}

pub struct TransformHandles {
    pub selected: Index,
    pub transform_type: TransformType,
    pub handles: Vec<TransformHandle>,
    pub mouse_hovering_on: Option<usize>,
    pub mouse_dragging_on: Option<usize>,
    initialized: bool,
}

impl TransformHandles {
    pub fn new(
        selected: Index,
        physics: &mut PhysicsContext,
        transform_type: TransformType,
    ) -> Self {
        let mut handles = Vec::new();

        for i in 0..HANDLE_AXES.len() {
            let collider = ColliderBuilder::ball(HANDLE_SIZE / 2.0)
                .collision_groups(HANDLE_INTERACTION_GROUP)
                .solver_groups(HANDLE_INTERACTION_GROUP)
                .build();

            let collider_handle = physics.collider_set.insert(collider);

            let transform_handle = TransformHandle {
                transform: TransformComponent::new(
                    Vec3::ZERO,
                    Quat::IDENTITY,
                    Vec3::splat(HANDLE_SIZE),
                ),
                collision_box: collider_handle,
                axis: HANDLE_AXES[i],
                color: HANDLE_AXES_COLOR[i],
            };

            handles.push(transform_handle)
        }

        Self {
            selected,
            transform_type,
            handles,
            initialized: false,
            mouse_hovering_on: None,
            mouse_dragging_on: None,
        }
    }

    fn handle_dragging(&mut self, game: &mut GameContext, transform: &mut TransformComponent) {
        let cursor_position = game.input_state.mouse_position;
        let window_extent = game.window.inner_size();

        let (origin, direction) = game.render_scene.camera.screen_to_world_ray(
            cursor_position.x,
            cursor_position.y,
            window_extent.width as f32,
            window_extent.height as f32,
        );

        let dragging_axis = self.mouse_dragging_on.unwrap();
        let dragging_axis = HANDLE_AXES[dragging_axis];

        let object_axis = transform.orientation * dragging_axis;

        let camera = &game.render_scene.camera;

        let screen_space = camera
            .world_to_screen_space(
                object_axis + transform.position,
                window_extent.width as f32,
                window_extent.height as f32,
            )
            .xy()
            - camera
                .world_to_screen_space(
                    transform.position,
                    window_extent.width as f32,
                    window_extent.height as f32,
                )
                .xy();

        let mut mouse_movement = game.input_state.mouse_delta.xy();
        mouse_movement.y = mouse_movement.y;

        let dot = screen_space.dot(mouse_movement);

        transform.position += object_axis * dot * DRAGGER_SENSITIVITY;
    }

    pub fn update(&mut self, game: &mut GameContext, transform: &mut TransformComponent) {
        assert_eq!(self.initialized, true, "handle must be initialized");

        let cursor_position = game.input_state.mouse_position;
        let window_extent = game.window.inner_size();

        let (origin, direction) = game.render_scene.camera.screen_to_world_ray(
            cursor_position.x,
            cursor_position.y,
            window_extent.width as f32,
            window_extent.height as f32,
        );

        let physics = &mut game.physics_context;
        let query_pipeline = physics.broad_phase.as_query_pipeline(
            physics.narrow_phase.query_dispatcher(),
            &physics.rigid_body_set,
            &physics.collider_set,
            QueryFilter::default(),
        );

        let ray = Ray::new(origin, direction);
        let query_result = query_pipeline.cast_ray(&ray, 2000.0, true);

        if let Some((handle, toi)) = query_result {
            let (ty, index, extra_data) =
                read_userdata(physics.collider_set.get(handle).unwrap().user_data);

            if ty == UserdataType::Handle as u8 {
                self.mouse_hovering_on = Some(extra_data as usize);
            } else {
                self.mouse_hovering_on = None;
            }
        }

        for handle in self.handles.iter_mut() {
            let direction = transform.orientation * handle.axis;

            let handle_position = transform.position
                + direction * ((transform.size * handle.axis).length() / 2.0 + HANDLE_OFFSET);

            let handle_orientation = Quat::from_rotation_arc(Vec3::Y, direction);

            handle.transform.position = handle_position;
            handle.transform.orientation = handle_orientation;

            let collider = physics.collider_set.get_mut(handle.collision_box).unwrap();
            collider.set_position(handle.transform.as_pose());
        }

        if self.mouse_hovering_on.is_some() && self.mouse_dragging_on.is_none() {
            if game.input_state.left_mouse_down {
                self.mouse_dragging_on = self.mouse_hovering_on;
            }
        }

        if self.mouse_dragging_on.is_some() {
            if game.input_state.left_mouse_down {
                self.handle_dragging(game, transform);
            } else {
                self.mouse_dragging_on = None;
            }
        }
    }

    pub fn init_colliders(&mut self, physics: &mut PhysicsContext, index: u64) {
        for (i, handle) in self.handles.iter_mut().enumerate() {
            let collider = physics.collider_set.get_mut(handle.collision_box).unwrap();
            collider.user_data = gen_userdata(crate::physics::UserdataType::Handle, index, i as u32)
        }

        self.initialized = true
    }

    pub fn draw(&self, render_scene: &mut RenderScene, transform: &TransformComponent) {
        let mesh_id = match self.transform_type {
            TransformType::Position => CONE_MESH_ID,
            TransformType::Size => SPHERE_MESH_ID,
            TransformType::Rotation => unimplemented!(),
        };

        for (i, handle) in self.handles.iter().enumerate() {
            render_scene.meshes.push(MeshNode {
                position: handle.transform.position,
                orientation: handle.transform.orientation,
                size: handle.transform.size,
                color: handle.color,
                opacity: match self.mouse_dragging_on.or(self.mouse_hovering_on) {
                    Some(index) if index == i => HANDLE_OPACITY_HOVERING,
                    _ => HANDLE_OPACITY,
                },
                mesh_id: mesh_id,
                is_gizmo: true,
                ..Default::default()
            });
        }
    }
}
