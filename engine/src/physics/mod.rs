use rapier3d::geometry::{Group, InteractionGroups, InteractionTestMode};

pub const PART_GROUP: Group = Group::GROUP_1;
pub const CHARACTER_GROUP: Group = Group::GROUP_2;
pub const GIZMO_GROUP: Group = Group::GROUP_9;

pub const PART_INTERACTION_GROUP: InteractionGroups = InteractionGroups::new(
    PART_GROUP,
    PART_GROUP.union(CHARACTER_GROUP),
    InteractionTestMode::And,
);

pub const CHARACTER_INTERACTION_GROUP: InteractionGroups = InteractionGroups::new(
    CHARACTER_GROUP,
    PART_GROUP.union(CHARACTER_GROUP),
    InteractionTestMode::And,
);

pub const GIZMO_INTERACTION_GROUP: InteractionGroups =
    InteractionGroups::new(GIZMO_GROUP, GIZMO_GROUP, InteractionTestMode::And);

pub mod context;
