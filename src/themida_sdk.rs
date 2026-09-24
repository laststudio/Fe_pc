// ==============================================================================
// Themida / SecureEngine SDK Rust 绑定与分级虚拟机保护宏封装
// 支持 Themida 3.2.4.52 及更高版本
// 分级虚拟机保护策略：
//   Tier 1: LION64 (White)  - 最关键代码 (鉴权、Token生成、硬件指纹哈希)
//   Tier 2: TIGER64 (Red)   - 核心业务/算法 (ETS MD5签名、打分评测XML、做题解析)
//   Tier 3: TIGER64 (White) - 普通业务/解析 (本地作业扫描提取、答案格式化导出)
//   Tier 4: FALCON64 (Tiny) - 外围代码 (状态装配、验证码参数、界面过滤与配置存取)
// ==============================================================================

#![allow(dead_code)]
#![allow(unused_macros)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

#[cfg(all(windows, not(test)))]
#[link(name = "SecureEngineSDK64", kind = "dylib")]
unsafe extern "system" {
    // 默认/通用虚拟机宏 (对应 fepc.tmd 中 OPTION_VIRTUAL_MACHINE_OLD_VM_MACROS_ID=0x28E9D505)
    #[link_name = "VMStart"]
    pub fn VM_START();
    #[link_name = "VMEnd"]
    pub fn VM_END();

    // 1. Tier 1: 最关键核心代码 - LION64 (White) (MachineId: 0x28959A01)
    #[link_name = "CustomVM00000161_Start"]
    pub fn VM_LION_WHITE_START();
    #[link_name = "CustomVM00000161_End"]
    pub fn VM_LION_WHITE_END();

    // 2. Tier 2: 较关键代码 - TIGER64 (Red) (MachineId: 0x28E9D505)
    #[link_name = "CustomVM00000104_Start"]
    pub fn VM_TIGER_RED_START();
    #[link_name = "CustomVM00000104_End"]
    pub fn VM_TIGER_RED_END();

    // 3. Tier 3: 不怎么关键代码 - TIGER64 (White) (MachineId: 0x28E9D501)
    #[link_name = "CustomVM00000103_Start"]
    pub fn VM_TIGER_WHITE_START();
    #[link_name = "CustomVM00000103_End"]
    pub fn VM_TIGER_WHITE_END();

    // 4. Tier 4: 其他剩下/外围代码 - FALCON64 (Tiny) (MachineId: 0x080AA202)
    #[link_name = "CustomVM00000218_Start"]
    pub fn VM_FALCON_TINY_START();
    #[link_name = "CustomVM00000218_End"]
    pub fn VM_FALCON_TINY_END();

    // 字符串加密宏
    #[link_name = "StrEncryptStart"]
    pub fn STR_ENCRYPT_START();
    #[link_name = "StrEncryptEnd"]
    pub fn STR_ENCRYPT_END();

    // 代码变异宏 (零解释器开销，抗反编译)
    #[link_name = "MutateStart"]
    pub fn VM_MUTATE_ONLY_START();
    #[link_name = "MutateEnd"]
    pub fn VM_MUTATE_ONLY_END();

    // 反调试与运行时完整性检测
    #[link_name = "SECheckDebugger"]
    pub fn CHECK_DEBUGGER(var: &mut i32, val: i32);

    #[link_name = "SECheckProtection"]
    pub fn CHECK_PROTECTION(var: &mut i32, val: i32);

    #[link_name = "SECheckCodeIntegrity"]
    pub fn CHECK_CODE_INTEGRITY(var: &mut i32, val: i32);
}

#[cfg(any(not(windows), test))]
pub unsafe fn VM_START() {}
#[cfg(any(not(windows), test))]
pub unsafe fn VM_END() {}
#[cfg(any(not(windows), test))]
pub unsafe fn VM_LION_WHITE_START() {}
#[cfg(any(not(windows), test))]
pub unsafe fn VM_LION_WHITE_END() {}
#[cfg(any(not(windows), test))]
pub unsafe fn VM_TIGER_RED_START() {}
#[cfg(any(not(windows), test))]
pub unsafe fn VM_TIGER_RED_END() {}
#[cfg(any(not(windows), test))]
pub unsafe fn VM_TIGER_WHITE_START() {}
#[cfg(any(not(windows), test))]
pub unsafe fn VM_TIGER_WHITE_END() {}
#[cfg(any(not(windows), test))]
pub unsafe fn VM_FALCON_TINY_START() {}
#[cfg(any(not(windows), test))]
pub unsafe fn VM_FALCON_TINY_END() {}
#[cfg(any(not(windows), test))]
pub unsafe fn STR_ENCRYPT_START() {}
#[cfg(any(not(windows), test))]
pub unsafe fn STR_ENCRYPT_END() {}
#[cfg(any(not(windows), test))]
pub unsafe fn VM_MUTATE_ONLY_START() {}
#[cfg(any(not(windows), test))]
pub unsafe fn VM_MUTATE_ONLY_END() {}
#[cfg(any(not(windows), test))]
pub unsafe fn CHECK_DEBUGGER(var: &mut i32, val: i32) {
    *var = val;
}
#[cfg(any(not(windows), test))]
pub unsafe fn CHECK_PROTECTION(var: &mut i32, val: i32) {
    *var = val;
}
#[cfg(any(not(windows), test))]
pub unsafe fn CHECK_CODE_INTEGRITY(var: &mut i32, val: i32) {
    *var = val;
}

// ------------------------------------------------------------------------------
// 安全且符合 Rust 惯例的分级虚拟机代码块保护宏
// ------------------------------------------------------------------------------

/// Tier 1: LION64 (White) - 最关键代码 (鉴权、Token生成、硬件指纹哈希)
#[macro_export]
macro_rules! themida_lion_white {
    ($($body:tt)*) => {{
        #[allow(unused_unsafe)]
        unsafe { $crate::themida_sdk::VM_LION_WHITE_START(); }
        let result = { $($body)* };
        #[allow(unused_unsafe)]
        unsafe { $crate::themida_sdk::VM_LION_WHITE_END(); }
        result
    }};
}

/// Tier 2: TIGER64 (Red) - 核心业务/算法 (ETS MD5签名、打分评测XML、做题解析)
#[macro_export]
macro_rules! themida_tiger_red {
    ($($body:tt)*) => {{
        #[allow(unused_unsafe)]
        unsafe { $crate::themida_sdk::VM_TIGER_RED_START(); }
        let result = { $($body)* };
        #[allow(unused_unsafe)]
        unsafe { $crate::themida_sdk::VM_TIGER_RED_END(); }
        result
    }};
}

/// Tier 3: TIGER64 (White) - 普通业务/解析 (本地作业扫描提取、答案格式化导出)
#[macro_export]
macro_rules! themida_tiger_white {
    ($($body:tt)*) => {{
        #[allow(unused_unsafe)]
        unsafe { $crate::themida_sdk::VM_TIGER_WHITE_START(); }
        let result = { $($body)* };
        #[allow(unused_unsafe)]
        unsafe { $crate::themida_sdk::VM_TIGER_WHITE_END(); }
        result
    }};
}

/// Tier 4: FALCON64 (Tiny) - 外围代码 (状态装配、验证码参数、界面过滤与配置存取)
#[macro_export]
macro_rules! themida_falcon_tiny {
    ($($body:tt)*) => {{
        #[allow(unused_unsafe)]
        unsafe { $crate::themida_sdk::VM_FALCON_TINY_START(); }
        let result = { $($body)* };
        #[allow(unused_unsafe)]
        unsafe { $crate::themida_sdk::VM_FALCON_TINY_END(); }
        result
    }};
}

/// 通用/兼容虚拟机宏 (默认路由至 Tier 2: TIGER64 Red)
#[macro_export]
macro_rules! themida_vm {
    ($($body:tt)*) => {
        $crate::themida_tiger_red!($($body)*)
    };
}

/// 兼容别名宏：统一路由至 themida_tiger_red!
#[macro_export]
macro_rules! themida_fish_black {
    ($($body:tt)*) => {
        $crate::themida_tiger_red!($($body)*)
    };
}

/// 兼容别名宏：统一路由至 themida_tiger_red!
#[macro_export]
macro_rules! themida_tiger_black {
    ($($body:tt)*) => {
        $crate::themida_tiger_red!($($body)*)
    };
}

/// 代码变异混淆宏 (零解释开销，抗反编译)
#[macro_export]
macro_rules! themida_mutate {
    ($($body:tt)*) => {{
        #[allow(unused_unsafe)]
        unsafe { $crate::themida_sdk::VM_MUTATE_ONLY_START(); }
        let result = { $($body)* };
        #[allow(unused_unsafe)]
        unsafe { $crate::themida_sdk::VM_MUTATE_ONLY_END(); }
        result
    }};
}

/// 敏感字符串内存动态解密宏
#[macro_export]
macro_rules! themida_str_encrypt {
    ($($body:tt)*) => {{
        #[allow(unused_unsafe)]
        unsafe { $crate::themida_sdk::STR_ENCRYPT_START(); }
        let result = { $($body)* };
        #[allow(unused_unsafe)]
        unsafe { $crate::themida_sdk::STR_ENCRYPT_END(); }
        result
    }};
}
