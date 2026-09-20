use rapier3d::geometry::{Group, InteractionGroups, InteractionTestMode};
use thunderdome::Index;

pub const PART_GROUP: Group = Group::GROUP_1;
pub const CHARACTER_GROUP: Group = Group::GROUP_2;
pub const HANDLE_GROUP: Group = Group::GROUP_9;

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

pub const HANDLE_INTERACTION_GROUP: InteractionGroups =
    InteractionGroups::new(HANDLE_GROUP, Group::NONE, InteractionTestMode::And);

// physics userdata stored in a u128
// [127:119] userdata type (8 bits)
// [119:96]  pad, reserved (24 bits)
// [95:64]   extra data    (32 bits)
// [63:0]    index slot      (64 bits)

pub enum UserdataType {
    Part = 1,
    Handle = 2,
}

pub const USERDATA_TYPE_PART: u128 = 1 << 127;
pub const USERDATA_TYPE_HANDLE: u128 = 1 << 127;

pub const fn gen_userdata(ty: UserdataType, index: u64, extra_data: u32) -> u128 {
    (ty as u128) << 119 | (extra_data as u128) << 64 | index as u128
}

pub const fn read_userdata(userdata: u128) -> (u8, u64, u32) {
    let ty = userdata >> 119;
    let extra_data = (userdata >> 64) & ((1 << 64) - 1);
    let index = userdata & ((1 << 64) - 1);
    (ty as u8, index as u64, extra_data as u32)
}

pub mod context;
