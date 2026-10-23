//! `TournamentMode` Lua singleton (HDR-style) for Midwest SSS integration.
//!
//! Borrowed registration pattern from HDR / stage-alts (`push_new_singleton`).

#![allow(improper_ctypes)]

use crate::css_first;
use crate::logutil;
use crate::ruleset;
use crate::scoreboard;
use crate::set_state;
use crate::strike;
use rlua_lua53_sys as lua;

unsafe fn push_new_singleton(
    lua_state: *mut lua::lua_State,
    name: &'static str,
    registry: &[lua::luaL_Reg],
) {
    let real_name = format!("{}\0", name);
    let meta_name = format!("Metatable{}\0", name);
    lua::luaL_newmetatable(lua_state, meta_name.as_ptr() as _);
    lua::lua_pushvalue(lua_state, -1);
    lua::lua_setfield(lua_state, -2, "__index\0".as_ptr() as _);

    lua::luaL_setfuncs(lua_state, registry.as_ptr(), 0);
    lua::lua_pop(lua_state, 1);

    lua::lua_newtable(lua_state);
    lua::lua_getfield(lua_state, lua::LUA_REGISTRYINDEX, meta_name.as_ptr() as _);
    lua::lua_setmetatable(lua_state, -2);

    let global_table = lua::bindings::index2addr(lua_state, lua::LUA_REGISTRYINDEX);
    let table = (*global_table).value.ptr;
    let value = if *(table as *mut u32).add(3) < 2 {
        // Empty registry edge case — skip register this frame.
        lua::lua_pop(lua_state, 1);
        return;
    } else {
        (*(table as *mut *mut lua::bindings::TValue).add(2)).add(1)
    };
    lua::bindings::auxsetstr(lua_state, value, real_name.as_ptr() as _);
}

extern "C" {
    fn add_to_key_context(ctx: &skyline::hooks::InlineCtx);
}

#[skyline::hook(replace = add_to_key_context)]
unsafe fn add_to_key_context_hook(ctx: &skyline::hooks::InlineCtx) {
    call_original!(ctx);

    let lua_state: *mut lua::lua_State = ctx.registers[19].x() as _;
    let registry = &[
        lua::luaL_Reg {
            name: "is_present\0".as_ptr() as _,
            func: Some(tm_is_present),
        },
        lua::luaL_Reg {
            name: "is_active\0".as_ptr() as _,
            func: Some(tm_is_active),
        },
        lua::luaL_Reg {
            name: "get_set_length\0".as_ptr() as _,
            func: Some(tm_get_set_length),
        },
        lua::luaL_Reg {
            name: "get_score_p1\0".as_ptr() as _,
            func: Some(tm_get_score_p1),
        },
        lua::luaL_Reg {
            name: "get_score_p2\0".as_ptr() as _,
            func: Some(tm_get_score_p2),
        },
        lua::luaL_Reg {
            name: "get_score_line\0".as_ptr() as _,
            func: Some(tm_get_score_line),
        },
        lua::luaL_Reg {
            name: "get_games_played\0".as_ptr() as _,
            func: Some(tm_get_games_played),
        },
        lua::luaL_Reg {
            name: "get_strike_controller\0".as_ptr() as _,
            func: Some(tm_get_strike_controller),
        },
        lua::luaL_Reg {
            name: "get_prompt\0".as_ptr() as _,
            func: Some(tm_get_prompt),
        },
        lua::luaL_Reg {
            name: "record_ban\0".as_ptr() as _,
            func: Some(tm_record_ban),
        },
        lua::luaL_Reg {
            name: "is_css_first\0".as_ptr() as _,
            func: Some(tm_is_css_first),
        },
        lua::luaL_Reg {
            name: "get_strike_preset\0".as_ptr() as _,
            func: Some(tm_get_strike_preset),
        },
        lua::luaL_Reg {
            name: std::ptr::null(),
            func: None,
        },
    ];

    push_new_singleton(lua_state, "TournamentMode", registry);
}

extern "C" fn tm_is_present(state: *mut lua::lua_State) -> i32 {
    unsafe {
        lua::lua_pushboolean(state, strike::api_is_present() as i32);
        1
    }
}

extern "C" fn tm_is_active(state: *mut lua::lua_State) -> i32 {
    unsafe {
        lua::lua_pushboolean(state, strike::api_is_active() as i32);
        1
    }
}

extern "C" fn tm_get_set_length(state: *mut lua::lua_State) -> i32 {
    unsafe {
        lua::lua_pushinteger(state, ruleset::set_length() as i64);
        1
    }
}

extern "C" fn tm_get_score_p1(state: *mut lua::lua_State) -> i32 {
    unsafe {
        lua::lua_pushinteger(state, set_state::snapshot().wins[0] as i64);
        1
    }
}

extern "C" fn tm_get_score_p2(state: *mut lua::lua_State) -> i32 {
    unsafe {
        lua::lua_pushinteger(state, set_state::snapshot().wins[1] as i64);
        1
    }
}

extern "C" fn tm_get_score_line(state: *mut lua::lua_State) -> i32 {
    unsafe {
        let line = scoreboard::hud_text();
        let c = std::ffi::CString::new(line).unwrap_or_default();
        lua::lua_pushstring(state, c.as_ptr());
        1
    }
}

extern "C" fn tm_get_games_played(state: *mut lua::lua_State) -> i32 {
    unsafe {
        lua::lua_pushinteger(state, set_state::snapshot().games_played as i64);
        1
    }
}

extern "C" fn tm_get_strike_controller(state: *mut lua::lua_State) -> i32 {
    unsafe {
        // 0 = ungated (game 1 / IRL ban order); 1/2 = active striking port.
        let c = strike::active_controller().map(|p| (p + 1) as i64).unwrap_or(0);
        lua::lua_pushinteger(state, c);
        1
    }
}

extern "C" fn tm_get_prompt(state: *mut lua::lua_State) -> i32 {
    unsafe {
        let prompt = strike::refresh_prompt();
        let c = std::ffi::CString::new(prompt).unwrap_or_default();
        lua::lua_pushstring(state, c.as_ptr());
        1
    }
}

extern "C" fn tm_record_ban(state: *mut lua::lua_State) -> i32 {
    unsafe {
        let picking = strike::record_ban();
        lua::lua_pushboolean(state, picking as i32);
        1
    }
}

extern "C" fn tm_is_css_first(state: *mut lua::lua_State) -> i32 {
    unsafe {
        lua::lua_pushboolean(state, css_first::is_css_first_active() as i32);
        1
    }
}

extern "C" fn tm_get_strike_preset(state: *mut lua::lua_State) -> i32 {
    unsafe {
        let preset = strike::snapshot().preset;
        let c = std::ffi::CString::new(preset).unwrap_or_default();
        lua::lua_pushstring(state, c.as_ptr());
        1
    }
}

pub fn install() {
    skyline::install_hooks!(add_to_key_context_hook);
    logutil::write("lua: TournamentMode singleton hook installed");
}
