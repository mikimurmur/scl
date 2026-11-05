//! 获取模组中文名称的模块
use base64::prelude::*;

/// 获取模组中文名称，如果没有则为空字符串，名称来自 MCMOD - Minecraft 模组中文百科
pub fn get_mod_cname(modid: &str) -> String {
    // https://gitee.com/SteveXMH/scl-data/raw/master/mcmod/cname/chisel
    let modid = BASE64_URL_SAFE.encode(modid);
    if let Ok(response) = crate::http::get(format!(
        "https://gitee.com/SteveXMH/scl-data/raw/master/mcmod/cname/{modid}"
    ))
    .call()
    {
        if response.is_success() {
            response.into_string().unwrap_or_default()
        } else {
            String::new()
        }
    } else {
        String::new()
    }
}
