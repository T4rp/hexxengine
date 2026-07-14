mod game;
mod part;

use crate::game::Game;

fn main() {
    hexxengine::game::start_game_app::<Game>();
}
