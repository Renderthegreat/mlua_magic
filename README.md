# mlua_magic_macros ✨

Procedural macros that turn Rust types into friendly Lua UserData
without ceremony, tedium, or type conversion headaches.

This crate is designed to simplify integration with the
[`mlua`](https://crates.io/crates/mlua) scripting engine.

---

## Features

### Struct field exposure
````rust
#[derive(Clone)]
#[mlua_magic_macros::structure]
struct Player {
    name: String,
    hp: i32,
}
````

Lua can read fields directly:

````lua
print(player.name)
print(player.hp)
````

Setter support planned.

---

### Enum variant exposure
````rust
#[derive(Clone, Copy)]
#[mlua_magic_macros::enumeration]
enum Status {
    Idle,
    Busy,
}
````

Lua usage:

````lua
local s = Status.Idle
````

Variants containing data can be exposed via static methods.

---

### Method and constructor binding
````rust
#[mlua_magic_macros::implementation]
impl Player {
    pub fn new(name: String) -> Self { ... }
    pub fn is_alive(&self) -> bool { ... }
}
````

Lua:

````lua
local p = Player.new("Zelda")
print(p:is_alive())
````

---

### One-line glue to complete registration

````rust
mlua_magic_macros::compile!(Player, fields, methods);
````

Supported helper flags:
- fields
- methods
- variants

---

## Full Example

````rust
#[derive(Clone, Copy)]
#[mlua_magic_macros::enumeration]
enum PlayerStatus {
    Idle,
    Walking,
    Attacking,
}

mlua_magic_macros::compile!(PlayerStatus, variants);

#[derive(Clone)]
#[mlua_magic_macros::structure]
struct Player {
    name: String,
    hp: i32,
    status: PlayerStatus,
}

#[mlua_magic_macros::implementation]
impl Player {
    pub fn new(name: String) -> Self {
        Self { name, hp: 100, status: PlayerStatus::Idle }
    }

    pub fn take_damage(&mut self, amount: i32) {
        self.hp = (self.hp - amount).max(0);
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }
}

mlua_magic_macros::compile!(Player, fields, methods);
````

Lua:

````lua
local player = Player.new("LuaHero")
player:take_damage(30)
print(player.hp)
````

---

## Roadmap

Planned improvements:
* Setter support for struct fields
* Automatic constructors for enum variants with data
* Custom type conversion extensibility points

---

## License

MIT

---

## Contributing

Issues, ideas, and PRs are welcome!
