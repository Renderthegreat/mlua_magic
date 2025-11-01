#[cfg(test)]
pub mod example {
	use ::mlua::prelude::*;

	use ::mlua_magic_macros;

	use ::serde::{Serialize, };

	use ::tracing::*;

	#[derive(Serialize)]
	#[derive(Clone)]
	#[derive(Copy)]
	#[derive(Debug)]
	#[mlua_magic_macros::enumeration]
	pub enum PlayerStatus {
		Idle,
		Walking,
		Attacking,
	}
	
	mlua_magic_macros::compile!(PlayerStatus, variants);

	#[derive(Serialize)]
	#[derive(Clone)]
	#[derive(Debug)]
	#[mlua_magic_macros::structure]
	pub struct Player {
		name: String,
		hp: i32,
		status: PlayerStatus,
	}

	#[mlua_magic_macros::implementation]
	impl Player {
		// This will be registered as a static "constructor"
		pub fn new(name: String) -> Self {
			return Self {
				name: name,
				hp: 100,
				status: PlayerStatus::Idle,
			};
		}

		// This will be registered as a `&mut self` method
		pub fn take_damage(&mut self, amount: i32) -> () {
			self.hp -= amount;
			if self.hp < 0 {
				self.hp = 0;
			};

			println!("Player took {} damage, {} HP remaining", amount, self.hp);
		}

		// This will be registered as a `&self` method
		pub fn is_alive(&self) -> bool {
			return self.hp > 0;
		}
	}

	mlua_magic_macros::compile!(Player, fields, methods);

	#[test]
	fn main() -> LuaResult<()> {
		::tracing_subscriber::fmt::init();
		let lua = Lua::new();

		// --- We can now call Player.new() FROM LUA! ---
		// We must register the type "constructor" with Lua first
		lua.globals().set("Player", lua.create_function(|_: & Lua, name: String| {
			return Ok(Player::new(name));
		})?)?;

		// --- This is the Lua script we will run ---
		let lua_script: &str = r#"
			-- Call the static `new` function we registered
			local player = Player("LuaHero");
			print("Player created:");
			print(player);

			-- Our derive macro automatically created these methods!
			print("Player name:", player.name);
			print("Player HP:", player.hp);
			print("Player status:", player.status);
			print("Is alive?", player:is_alive());

			-- Call our new custom method
			player:take_damage(30);
			
			print("-----------------------------------")
			print("New player HP:", player.hp)

			-- Call the method again
			player:take_damage(80);
			print("Player HP after final hit:", player.hp);
			print("Is alive?", player:is_alive());
		"#;

		// Execute the script
		lua.load(lua_script).exec()?;

		// We can also retrieve the player and see the changes reflected in Rust
		// let modified_player: Player = lua.globals().get("player")?;

		info!("\n--- Back in Rust ---");
		// println!("Player after Lua script: {:?}", modified_player);

		// assert_eq!(modified_player.hp, 0);
		// assert_eq!(modified_player.status, PlayerStatus::Attacking);
		// assert_eq!(modified_player.is_alive(), false);

		Ok(())
	}


}