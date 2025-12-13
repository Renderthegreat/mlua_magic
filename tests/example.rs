#[cfg(test)]
pub mod example {
	use ::mlua::prelude::*;

	use ::mlua_magic_macros;

	use ::smol::Timer;
	use ::std::time::Duration;
	use ::tracing::*;

	#[derive(Debug, Copy, Clone, Default, PartialEq)]
	#[mlua_magic_macros::enumeration]
	pub enum PlayerStatus {
		#[default]
		Idle,
		Walking,
		Attacking(i32),
	}

	mlua_magic_macros::compile!(type_path = PlayerStatus, variants = true);

	#[derive(Debug, Clone, Default)]
	#[mlua_magic_macros::structure]
	pub struct Player {
		name: String,
		hp: i32,
		status: PlayerStatus,
	}

	mlua_magic_macros::compile!(type_path = Player, fields = true, methods = true);

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

	#[derive(Debug, Clone, Default)]
	#[mlua_magic_macros::structure]
	pub struct Db {
		count: i32,
	}

	#[mlua_magic_macros::implementation]
	impl Db {
		pub async fn init_async() -> Self {
			// Simulate initialization
			Timer::after(Duration::from_millis(100)).await;
			return Self { count: 0 };
		}

		pub async fn write_async(&mut self) -> () {
			// Simulate write
			Timer::after(Duration::from_millis(100)).await;
			self.count += 1;
		}

		pub async fn read_async(&self) -> i32 {
			// Simulate read
			Timer::after(Duration::from_millis(100)).await;
			return self.count;
		}
	}

	mlua_magic_macros::compile!(type_path = Db, fields = false, methods = true);

	#[test]
	fn main() -> LuaResult<()> {
		smol::block_on(async {
			::tracing_subscriber::fmt::init();
			let lua: Lua = Lua::new();

			// # We can now call Player.new() FROM LUA! ---
			mlua_magic_macros::load!(lua, Player, PlayerStatus, Db);

			// # This is the Lua script we will run ---
			let lua_script: &str = r#"
                -- Call the static `new` function we registered
                print(PlayerStatus.Idle());
                player = Player.new("LuaHero");
                print("Player created:");

                -- Our derive macro automatically created these methods!
                print("Player name:", player.name);
                print("Player HP:", player.hp);
                print("Player status:", player.status);
                print("Is alive?", player:is_alive());

                -- Call our new custom method
                player:take_damage(30);
                player.status = PlayerStatus.Attacking(20);
                
                print("-----------------------------------");
                print("New player HP:", player.hp)

                -- Call the method again
                print("-----------------------------------");
                player:take_damage(80);
                print("Player HP after final hit:", player.hp);
                print("Player status:", player.status);
                print("Is alive?", player:is_alive());

                -- Now let's test some async methods
                print("-----------------------------------");
                -- All of these functions will yield
                db = Db.init_async();
                print("Db created");

                print("-----------------------------------");
                print("Initial value:", db:read_async());
                db:write_async(); 
                print("New value:", db:read_async());
            "#;

			// Execute the script
			lua.load(lua_script).exec_async().await?;

			// We can also retrieve the player and see the changes reflected in Rust
			let modified_player: Player = lua.globals().get("player")?;
			let modified_db: Db = lua.globals().get("db")?;
			modified_db.read_async().await;

			info!("\n--- Back in Rust ---");
			info!("Player after Lua script: {:?}", modified_player);
			info!("Db after Lua script: {:?}", modified_db);

			assert_eq!(modified_player.hp, 0);
			assert_eq!(modified_player.status, PlayerStatus::Attacking(20));
			assert!(!modified_player.is_alive());

			assert_eq!(modified_db.count, 1);

			return Ok(());
		})
	}
}
