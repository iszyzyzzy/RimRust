use super::*;

use ahash::{HashMap, HashSet, HashMapExt, HashSetExt};

// 这几位是太爷，写xml的时候还得加到knownExpansions里
// About.xml里还没有name的
// 还有一个core也不带name就算了吧，读到了手动加
use once_cell::sync::Lazy;
pub static DLC_LIST: Lazy<HashMap<PackageId, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(
        PackageId::from_str("ludeon.rimworld.royalty"),
        "Rimworld - Royalty",
    );
    m.insert(
        PackageId::from_str("ludeon.rimworld.ideology"),
        "Rimworld - Ideology",
    );
    m.insert(
        PackageId::from_str("ludeon.rimworld.biotech"),
        "Rimworld - Biotech",
    );
    m.insert(
        PackageId::from_str("ludeon.rimworld.anomaly"),
        "Rimworld - Anomaly",
    );
    m.insert(
        PackageId::from_str("ludeon.rimworld.odyssey"),
        "Rimworld - Odyssey",
    );
    m
});

// 预定义first和last
pub static FIRST_MOD: Lazy<HashSet<PackageId>> = Lazy::new(|| {
    let mut s = HashSet::new();
    s.insert(PackageId::from_str("zetrith.prepatcher"));
    s.insert(PackageId::from_str("brrainz.harmony"));
    s.insert(PackageId::from_str("ludeon.rimworld"));
    s.insert(PackageId::from_str("ludeon.rimworld.royalty"));
    s.insert(PackageId::from_str("ludeon.rimworld.ideology"));
    s.insert(PackageId::from_str("ludeon.rimworld.biotech"));
    s.insert(PackageId::from_str("ludeon.rimworld.anomaly"));
    s.insert(PackageId::from_str("ludeon.rimworld.odyssey"));
    s.insert(PackageId::from_str("unlimitedhugs.hugslib"));
    s.insert(PackageId::from_str("bs.performance"));
    s.insert(PackageId::from_str("bs.fishery"));
    s
});
pub static LAST_MOD: Lazy<HashSet<PackageId>> = Lazy::new(|| {
    let mut s = HashSet::new();
    s.insert(PackageId::from_str("krkr.rocketman"));
    s
});

pub static LANGUAGE_CODE_TO_NAME: Lazy<HashMap<&'static str, (&'static str, &'static str)>> =
    Lazy::new(|| {
        let mut m = HashMap::new();
        m.insert("ca", ("Catalan", "Català"));
        m.insert("zh", ("ChineseSimplified", "简体中文"));
        m.insert("zh-TW", ("ChineseTraditional", "繁體中文"));
        m.insert("cs", ("Czech", "Čeština"));
        m.insert("da", ("Danish", "Dansk"));
        m.insert("nl", ("Dutch", "Nederlands"));
        m.insert("en", ("English", "English"));
        m.insert("et", ("Estonian", "Eesti"));
        m.insert("fi", ("Finnish", "Suomi"));
        m.insert("fr", ("French", "Français"));
        m.insert("de", ("German", "Deutsch"));
        m.insert("el", ("Greek", "Ελληνικά"));
        m.insert("hu", ("Hungarian", "Magyar"));
        m.insert("it", ("Italian", "Italiano"));
        m.insert("ja", ("Japanese", "日本語"));
        m.insert("ko", ("Korean", "한국어"));
        m.insert("no", ("Norwegian", "Norsk Bokmål"));
        m.insert("en-P", ("Pirate", "Pirate"));
        m.insert("pl", ("Polish", "Polski"));
        m.insert("pt", ("Portuguese", "Português"));
        m.insert("pt-BR", ("PortugueseBrazilian", "Português Brasileiro"));
        m.insert("ro", ("Romanian", "Română"));
        m.insert("ru", ("Russian", "Русский"));
        m.insert("sk", ("Slovak", "Slovenčina"));
        m.insert("sl", ("Slovenian", "slovenščina"));
        m.insert("es", ("Spanish", "Español(Castellano)"));
        m.insert("es-419", ("SpanishLatin", "Español(Latinoamérica)"));
        m.insert("sv", ("Swedish", "Svenska"));
        m.insert("tr", ("Turkish", "Türkçe"));
        m.insert("uk", ("Ukrainian", "Українська"));
        // 我不知道它是出于什么心态放进去的，但我看到这条panic的时候我要panic了
        m.insert("mar", ("火星文", "火星文"));
        // +1
        m.insert("rwd", ("RimWaldo", "림왈도"));
        m
    });

fn normalize_language_name(name: &str) -> String {
    name.trim().to_lowercase().replace(&['(', ')', ' '][..], "")
}

pub static LANGUAGE_NAME_TO_CODE: Lazy<HashMap<String, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // 从现有映射添加基本对应
    // 只要我塞的够多就不会有问题
    LANGUAGE_CODE_TO_NAME
        .iter()
        .for_each(|(code, (name, local))| {
            m.insert(name.to_string(), *code);
            m.insert(local.to_string(), *code);
            m.insert(normalize_language_name(name), *code);
            m.insert(normalize_language_name(local), *code);
            m.insert(format!("{}({})", name, local), *code);
            m.insert(format!("{} ({})", name, local), *code);
            m.insert(format!("{}({})", local, name), *code);
        });

    let extra_mappings = [
        ("Chineese", "zh"),
        ("Brazilian Portuguese", "pt-BR"),
        ("PortugueseBrazillian", "pt-BR"),
        ("TraditionalChinese", "zh-TW"),
        ("Ukraine", "uk"),
        ("Ukranian", "uk"),
    ];

    for (name, code) in extra_mappings {
        m.insert(name.to_string(), code);
        m.insert(normalize_language_name(name), code);
    }

    m
});