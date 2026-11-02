//! Trusted catalogue selection. A Mac 240 fps source must be retimed by the
//! asynchronous helper before it can be handed to the native TV decoder.
use serde::Deserialize;
use std::io::Read;
#[derive(Clone, Debug, Deserialize)]
pub struct Scene {
    pub id: String,
    pub title: String,
    pub category: String,
    pub variants: Vec<Variant>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variant {
    pub url: String,
    pub height: u32,
    pub range: String,
    #[serde(default)]
    pub dynamic_range: String,
    #[serde(default = "normal_fps")]
    pub fps: f64,
    pub available: Option<bool>,
}
fn normal_fps() -> f64 {
    30.0
}
#[derive(Clone, Copy, Default)]
pub struct Capabilities {
    pub dolby: bool,
    pub hdr: bool,
}
pub fn valid_scene_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 100
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}
pub fn load(path: &std::path::Path) -> Vec<Scene> {
    let Ok(file) = std::fs::File::open(path) else {
        return vec![];
    };
    let mut bytes = Vec::new();
    if file.take(4_194_305).read_to_end(&mut bytes).is_err() || bytes.len() > 4_194_304 {
        return vec![];
    }
    let Ok(values) = serde_json::from_slice::<Vec<serde_json::Value>>(&bytes) else {
        return vec![];
    };
    let mut seen = std::collections::HashSet::new();
    values
        .into_iter()
        .take(512)
        .filter_map(|v| serde_json::from_value::<Scene>(v).ok())
        .filter_map(|mut s| {
            if !valid_scene_id(&s.id) || !seen.insert(s.id.clone()) {
                return None;
            }
            s.title = s
                .title
                .chars()
                .filter(|c| !c.is_control())
                .take(160)
                .collect();
            s.category = s
                .category
                .chars()
                .filter(|c| !c.is_control())
                .take(40)
                .collect();
            s.variants.truncate(32);
            if s.title.is_empty() {
                return None;
            }
            Some(s)
        })
        .collect()
}
pub fn choose(scene: &Scene, mode: &str, caps: Capabilities, helper: &str) -> Vec<Variant> {
    if !valid_scene_id(&scene.id) {
        return vec![];
    }
    let mut seen = std::collections::HashSet::new();
    let mut variants: Vec<_> = scene
        .variants
        .iter()
        .filter(|v| {
            if !(1440..=4320).contains(&v.height)
                || !v.fps.is_finite()
                || v.fps <= 0.0
                || v.fps > 1000.0
                || v.available == Some(false)
                || !safe_apple_url(&v.url)
                || (v.fps > 60.0 && !valid_helper(helper))
            {
                return false;
            }
            let compatible = match v.dynamic_range.as_str() {
                "Dolby Vision" => v.range == "HDR" && caps.dolby && mode != "sdr" && mode != "hdr",
                "HDR10" | "HLG" => v.range == "HDR" && caps.hdr && mode != "sdr",
                "" | "SDR" => v.range == "SDR",
                _ => false,
            };
            compatible && seen.insert(v.url.clone())
        })
        .cloned()
        .collect();
    variants.sort_by_key(|v| {
        std::cmp::Reverse((
            v.height,
            match v.dynamic_range.as_str() {
                "Dolby Vision" => 3,
                "HDR10" | "HLG" => 2,
                _ => 1,
            },
            v.fps <= 60.0,
        ))
    });
    variants
}
pub fn safe_apple_url(url: &str) -> bool {
    if url.len() > 2048
        || url
            .bytes()
            .any(|b| b.is_ascii_control() || b.is_ascii_whitespace() || b == b'\\')
    {
        return false;
    }
    let lower = url.to_ascii_lowercase();
    if ["%00", "%0a", "%0d"].iter().any(|c| lower.contains(c)) {
        return false;
    }
    [
        "http://sylvan.apple.com/",
        "https://sylvan.apple.com/",
        "http://a1.phobos.apple.com/",
        "https://a1.phobos.apple.com/",
    ]
    .iter()
    .any(|h| url.strip_prefix(h).is_some_and(|path| !path.is_empty()))
}
pub fn valid_helper(value: &str) -> bool {
    if value.len() > 128 {
        return false;
    }
    let Some(authority) = value.strip_prefix("http://") else {
        return false;
    };
    let Some((host, port)) = authority.split_once(':') else {
        return false;
    };
    if port.is_empty()
        || !port.bytes().all(|b| b.is_ascii_digit())
        || port.parse::<u16>().ok().filter(|p| *p > 0).is_none()
    {
        return false;
    }
    if host == "localhost" {
        return true;
    }
    let Ok(ip) = host.parse::<std::net::Ipv4Addr>() else {
        return false;
    };
    ip.is_private() || ip.is_loopback()
}
pub fn playback_url(scene: &Scene, variant: &Variant, _helper: &str) -> String {
    if valid_scene_id(&scene.id)
        && safe_apple_url(&variant.url)
        && variant.height >= 1440
        && variant.fps.is_finite()
        && variant.fps > 0.0
        && variant.fps <= 60.0
    {
        variant.url.clone()
    } else {
        String::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn variant(path: &str, height: u32, range: &str, dynamic: &str, fps: f64) -> Variant {
        Variant {
            url: format!("https://sylvan.apple.com/{path}.mov"),
            height,
            range: range.into(),
            dynamic_range: dynamic.into(),
            fps,
            available: None,
        }
    }
    fn scene(variants: Vec<Variant>) -> Scene {
        Scene {
            id: "scene-one".into(),
            title: "Test".into(),
            category: "Cities".into(),
            variants,
        }
    }
    #[test]
    fn maximum_quality_respects_capabilities_and_never_treats_unverified_hdr_as_hdr10() {
        let s = scene(vec![
            variant("dv", 2160, "HDR", "Dolby Vision", 30.0),
            variant("hdr", 2160, "HDR", "HDR10", 30.0),
            variant("sdr", 2160, "SDR", "", 30.0),
            variant("unverified", 2160, "HDR", "", 30.0),
            variant("hd", 1080, "SDR", "", 30.0),
        ]);
        let v = choose(
            &s,
            "auto",
            Capabilities {
                dolby: true,
                hdr: true,
            },
            "",
        );
        assert_eq!(v.len(), 3);
        assert!(v[0].url.ends_with("/dv.mov"));
        let hdr = choose(
            &s,
            "hdr",
            Capabilities {
                dolby: true,
                hdr: true,
            },
            "",
        );
        assert_eq!(hdr.len(), 2);
        assert!(hdr[0].url.ends_with("/hdr.mov"));
        assert_eq!(
            choose(
                &s,
                "sdr",
                Capabilities {
                    dolby: true,
                    hdr: true
                },
                ""
            )
            .len(),
            1
        );
        assert_eq!(choose(&s, "auto", Capabilities::default(), "").len(), 1);
    }
    #[test]
    fn native_rate_precedes_retiming_and_a_helper_url_is_never_guessed() {
        let s = scene(vec![
            variant("mac", 2160, "SDR", "", 240.0),
            variant("native", 2160, "SDR", "", 30.0),
        ]);
        assert_eq!(choose(&s, "auto", Capabilities::default(), "").len(), 1);
        let v = choose(&s, "auto", Capabilities::default(), "http://localhost:8794");
        assert_eq!(v.len(), 2);
        assert!(v[0].url.ends_with("/native.mov"));
        assert!(playback_url(&s, &v[1], "http://localhost:8794").is_empty());
        assert!(playback_url(&s, &v[0], "").ends_with("/native.mov"));
    }
    #[test]
    fn helper_authorities_and_apple_urls_are_strictly_bounded() {
        for good in ["http://localhost:8794", "http://127.0.0.1:80"] {
            assert!(valid_helper(good), "{good}");
        }
        for ip in [
            std::net::Ipv4Addr::new(10, 1, 2, 3),
            std::net::Ipv4Addr::new(172, 31, 2, 3),
            std::net::Ipv4Addr::new(192, 168, 1, 2),
        ] {
            assert!(valid_helper(&format!("http://{ip}:8794")));
        }
        for bad in [
            "https://localhost:80",
            "http://example.com:80",
            "http://8.8.8.8:80",
            "http://172.32.0.1:8794",
            "http://localhost:0",
            "http://localhost:+80",
            "http://localhost:80/path",
            "http://user@localhost:80",
            "http://localhost:65536",
            "http://localhost:80?x",
        ] {
            assert!(!valid_helper(bad), "{bad}");
        }
        for bad in [
            "https://sylvan.apple.com.evil/a",
            "https://sylvan.apple.com@evil/a",
            "file:///a",
            "https://user@sylvan.apple.com/a",
            "http://sylvan.apple.com/\nHeader",
            "http://sylvan.apple.com/%0aHeader",
        ] {
            assert!(!safe_apple_url(bad), "{bad}");
        }
    }
    #[test]
    fn malformed_variants_cannot_overflow_priority_or_escape_scene_ids() {
        let s = scene(vec![
            variant("huge", u32::MAX, "SDR", "", 30.0),
            variant("nan", 2160, "SDR", "", f64::NAN),
            variant("negative", 2160, "SDR", "", -1.0),
            variant("safe", 1440, "SDR", "", 30.0),
        ]);
        assert_eq!(choose(&s, "auto", Capabilities::default(), "").len(), 1);
        let mut bad = s.clone();
        bad.id = "../other".into();
        assert!(choose(&bad, "auto", Capabilities::default(), "").is_empty());
    }
}
