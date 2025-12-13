# `mlua_magic_macros`

[Docs](https://crates.io/crates/mlua_magic_macros)

Simple, magical proc-macros to export Rust structs and enums to `mlua` with minimal boilerplate.

## 🚀 What It Does

This crate provides a set of attribute macros that write the "magic" glue code to automatically generate `impl mlua::UserData` for your Rust types.

  * Expose struct fields as Lua properties (`player.hp`).
  * Expose enum unit variants as Lua constructors (`PlayerStatus.Idle()`).
  * Expose Rust methods (`&self`, `&mut self`, and `static`) as Lua methods (`player:take_damage(10)`).

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
mlua = { version = "0.11.4", features = ["lua54", "macros"] } # Or the version you are using
mlua-magic-macros = "0.2.0" # Or the version you are using
```

## ✨ Quick Start Example

Here is a complete, copy-pasteable example.

### 1\. The Rust Code

Define your types and "decorate" them with the macros.

```rust
use ::mlua::prelude::*;
use ::mlua_magic_macros;

// STEP 1: Decorate your enum
#[derive(Debug, Copy, Clone, Default, PartialEq)]
#[mlua_magic_macros::enumeration]
pub enum PlayerStatus {
    #[default] Idle,
    Walking,
    Attacking,
}

// STEP 2: Compile the UserData impl for the enum
// This generates `impl mlua::UserData for PlayerStatus`
mlua_magic_macros::compile!(type_path = PlayerStatus, variants = true);


// STEP 1: Decorate your struct
#[derive(Debug, Clone, Default)]
#[mlua_magic_macros::structure]
pub struct Player {
    name: String,
    hp: i32,
    status: PlayerStatus,
}

// STEP 1 (continued): Decorate the impl block
#[mlua_magic_macros::implementation]
impl Player {
    // Registered as a static "constructor": Player.new()
    pub fn new(name: String) -> Self {
        return Self {
            name,
            hp: 100,
            status: PlayerStatus::Idle,
        };
    }

    // Registered as a `&mut self` method: player:take_damage()
    pub fn take_damage(&mut self, amount: i32) -> () {
        self.hp -= amount;
        if self.hp < 0 {
            self.hp = 0;
        };
    }

    // Registered as a `&self` method: player:is_alive()
    pub fn is_alive(&self) -> bool {
        return self.hp > 0;
    }
}

// STEP 2: Compile the UserData impl for the struct
// This generates `impl mlua::UserData for Player`
mlua_magic_macros::compile!(type_path = Player, fields = true, methods = true);


// STEP 3: Load the types into a Lua instance
fn main() -> LuaResult<()> {
    let lua = Lua::new();

    // This makes `Player` and `PlayerStatus` available as globals in Lua
    mlua_magic_macros::load!(lua, Player, PlayerStatus);

    // See the Lua script below!
    run_lua_code(&lua)?;
    return Ok(());
}
```

### 2\. The Lua Script

Now you can call your Rust code *from* Lua as if it were a native Lua table.

```lua
-- run_lua_code.lua

-- Call the static `new` function
local player = Player.new("LuaHero");
print("Player created:");

-- Access struct fields directly (from `#[structure]`)
print("Player name:", player.name);
print("Player HP:", player.hp);

-- Access enum variants (from `#[enumeration]`)
print("Player status:", player.status); -- "Idle"
print("Is alive?", player:is_alive()); -- `true`

-- Call a `&mut self` method
player:take_damage(30);
print("New player HP:", player.hp); -- 70

-- You can even set fields!
player.status = PlayerStatus.Attacking();
print("Player status:", player.status); -- "Attacking"

player:take_damage(80)
print("Player HP after final hit:", player.hp); -- 0
print("Is alive?", player:is_alive()); -- `false`
```

## 📚 API Guide

This crate uses a 3-step process:

1.  **Decorate:** Add attributes (`#[...]`) to your types to tell the macros what to export.
2.  **Compile:** Use the `compile!(...)` macro to generate the `impl mlua::UserData` block.
3.  **Load:** Use the `load!(...)` macro at runtime to register your types with a `Lua` instance. You can also manually load your types if our API doesn't allow you to do something.

### Step 1: Decorate

| Macro | Target | Purpose |
| :--- | :--- | :--- |
| `#[enumeration]` | `enum` | Exposes **unit variants** as static functions (e.g., `MyEnum.VariantA()`). |
| `#[structure]` | `struct`| Exposes **fields** as readable/writable properties (e.g., `my_struct.field`). |
| `#[implementation]`| `impl` | Exposes **functions** as methods (e.g., `MyType.new()`, `my_inst:do_thing()`). |

### Step 2: Compile

The `compile!` macro generates the final `impl mlua::UserData` and `impl mlua::FromLua` for your type.

```rust
mlua_magic_macros::compile!(
    type_path = MyType, // The name of the struct/enum
    fields = true,      // Include fields from `#[structure]`?
    methods = true,     // Include methods from `#[implementation]`?
    variants = true     // Include variants from `#[enumeration]`?
);
```

### Step 3: Load

The `load!` macro registers your compiled types as globals in Lua.

```rust
let lua = Lua::new();
// This is like running:
// _G.Player = (proxy for Player UserData)
// _G.PlayerStatus = (proxy for PlayerStatus UserData)
mlua_magic_macros::load!(lua, Player, PlayerStatus);
```

## License

This crate is licensed under the **[MIT license](http://opensource.org/licenses/MIT)**.
