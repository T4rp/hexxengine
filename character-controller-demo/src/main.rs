mod components;
mod entities;
mod game;

use hexxengine::game::start_game_app;

use crate::game::Game;

fn main() {
    start_game_app::<Game>();
}
