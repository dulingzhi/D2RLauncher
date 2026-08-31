/// 按 uid 注册的游戏配置表
/// 新增游戏 = 在 GAMES 中追加一条 GameConfig

use std::path::PathBuf;

/// WoW 客户端分支
pub const FLAVOR_RETAIL: &str = "retail";
pub const FLAVOR_CLASSIC: &str = "classic";
pub const FLAVOR_CLASSIC_ERA: &str = "classic_era";
pub const FLAVOR_TITAN: &str = "titan";
pub const FLAVOR_ANNIVERSARY: &str = "anniversary";

pub struct GameConfig {
    /// 启动参数 -uid 的值，如 "osic" / "wow"
    pub uid: &'static str,
    /// Battle.net 应用代码：登录 URL 的 app= 与注册表键名，如 "OSI" / "WOW"
    pub code: &'static str,
    /// 界面显示名
    pub display_name: &'static str,
    /// 启动参数（自定义参数之后追加）
    pub launch_args: &'static [&'static str],
    /// 多开互斥锁名（D2R 专有，WOW 为 None）
    pub mutex_name: Option<&'static str>,
    /// 游戏窗口标题识别关键字
    pub window_title_keyword: &'static str,
}

pub fn games() -> &'static [GameConfig] {
    &GAMES
}

static GAMES: &[GameConfig] = &[
    GameConfig {
        uid: "osic",
        code: "OSI",
        display_name: "暗黑破坏神2：狱火重生",
        launch_args: &["-uid", "osic"],
        mutex_name: Some("DiabloII Check For Other Instances"),
        window_title_keyword: "Diablo II: Resurrected",
    },
    GameConfig {
        uid: "wow",
        code: "WOW",
        display_name: "魔兽世界",
        launch_args: &["-launcherlogin", "-uid", "wow"],
        mutex_name: None,
        window_title_keyword: "World of Warcraft",
    },
];

pub fn get_game(uid: &str) -> Option<&'static GameConfig> {
    GAMES.iter().find(|g| g.uid == uid)
}

impl GameConfig {
    /// 登录页 URL（externalChallenge，token 捕获用）
    pub fn login_url(&self) -> String {
        format!(
            "https://account.battlenet.com.cn/login/zh/?externalChallenge=login&app={}",
            self.code
        )
    }

    /// HKCU 下 token 写入的注册表路径
    pub fn registry_path(&self) -> String {
        format!(
            r"SOFTWARE\Blizzard Entertainment\Battle.net\Launch Options\{}",
            self.code
        )
    }

    /// 重命名窗口后的标题：账号名 + 英文关键字
    /// 关键字必须保留，否则改名后窗口不再被 matches_window_title 识别
    pub fn window_title_for(&self, account_label: &str) -> String {
        format!("{} - {}", account_label, self.window_title_keyword)
    }

    /// 相对安装根目录的候选 exe（按优先级）
    pub fn exe_candidates(&self, flavor: &str) -> Vec<PathBuf> {
        match self.uid {
            "osic" => vec![PathBuf::from("D2R.exe")],
            "wow" => match flavor {
                // 分支 exe 只认子目录路径：根目录裸 WowClassic.exe 无法区分
                // 各怀旧类分支，不作为候选
                FLAVOR_CLASSIC => vec![PathBuf::from(r"_classic_\WowClassic.exe")],
                FLAVOR_CLASSIC_ERA => vec![PathBuf::from(r"_classic_era_\WowClassic.exe")],
                FLAVOR_TITAN => vec![PathBuf::from(r"_classic_titan_\WowClassic.exe")],
                FLAVOR_ANNIVERSARY => vec![PathBuf::from(r"_anniversary_\WowClassic.exe")],
                // 默认正式服
                _ => vec![PathBuf::from(r"_retail_\Wow.exe")],
            },
            _ => vec![],
        }
    }

    /// 游戏的分支代码列表（探测用；osic 等无分支概念的游戏为空）
    pub fn flavors(&self) -> &'static [&'static str] {
        match self.uid {
            "wow" => &[
                FLAVOR_RETAIL,
                FLAVOR_CLASSIC,
                FLAVOR_CLASSIC_ERA,
                FLAVOR_TITAN,
                FLAVOR_ANNIVERSARY,
            ],
            _ => &[],
        }
    }

    /// 探测安装目录下已安装的分支（任一候选 exe 存在即算已装）
    pub fn installed_flavors(&self, game_path: &str) -> Vec<String> {
        let root = PathBuf::from(game_path.trim_end_matches('\\'));
        self.flavors()
            .iter()
            .filter(|f| self.exe_candidates(f).iter().any(|rel| root.join(rel).is_file()))
            .map(|f| f.to_string())
            .collect()
    }

    /// 在安装根目录下探测实际存在的 exe
    pub fn resolve_exe(&self, game_path: &str, flavor: &str) -> Result<PathBuf, String> {
        let root = game_path.trim_end_matches('\\');
        let candidates = self.exe_candidates(flavor);
        for rel in &candidates {
            let full = PathBuf::from(root).join(rel);
            if full.is_file() {
                return Ok(full);
            }
        }
        let tried = candidates
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("、");
        Err(format!(
            "未在 {} 安装目录找到客户端：{}（已尝试：{}）",
            self.display_name, root, tried
        ))
    }
}

/// 窗口标题是否属于任一已注册游戏（进程监控用）
pub fn matches_window_title(title: &str) -> bool {
    GAMES.iter().any(|g| title.contains(g.window_title_keyword))
}

/// 探测指定游戏已安装的分支（读设置里的安装目录）
/// 返回空 = 未配置目录 / 未安装 / 游戏无分支概念，前端均直接走启动流程
#[tauri::command]
pub fn get_installed_flavors(app: tauri::AppHandle, game: String) -> Result<Vec<String>, String> {
    let cfg = get_game(&game).ok_or_else(|| format!("不支持的游戏 uid: {}", game))?;
    let settings = crate::settings::get_settings(app);
    let path = crate::settings::game_path_for(&settings, cfg.uid);
    if path.trim().is_empty() {
        return Ok(vec![]);
    }
    Ok(cfg.installed_flavors(&path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 创建带占位 exe 的临时安装目录，返回根目录路径
    fn temp_install(tag: &str, exes: &[&str]) -> PathBuf {
        let root = std::env::temp_dir()
            .join("d2r-launcher-tests")
            .join(format!("{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for exe in exes {
            let p = root.join(exe);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, b"stub").unwrap();
        }
        root
    }

    #[test]
    fn d2r_config_uses_osi_code() {
        let g = get_game("osic").expect("osic 应已注册");
        assert_eq!(g.code, "OSI");
        assert_eq!(g.launch_args, &["-uid", "osic"]);
        assert_eq!(g.mutex_name, Some("DiabloII Check For Other Instances"));
    }

    #[test]
    fn wow_config_uses_wow_code_and_launcherlogin() {
        let g = get_game("wow").expect("wow 应已注册");
        assert_eq!(g.code, "WOW");
        assert_eq!(g.launch_args, &["-launcherlogin", "-uid", "wow"]);
        assert_eq!(g.mutex_name, None);
    }

    #[test]
    fn unknown_uid_is_rejected() {
        assert!(get_game("xyz").is_none());
    }

    #[test]
    fn login_url_derives_from_code() {
        assert_eq!(
            get_game("osic").unwrap().login_url(),
            "https://account.battlenet.com.cn/login/zh/?externalChallenge=login&app=OSI"
        );
        assert_eq!(
            get_game("wow").unwrap().login_url(),
            "https://account.battlenet.com.cn/login/zh/?externalChallenge=login&app=WOW"
        );
    }

    #[test]
    fn registry_path_derives_from_code() {
        assert_eq!(
            get_game("wow").unwrap().registry_path(),
            r"SOFTWARE\Blizzard Entertainment\Battle.net\Launch Options\WOW"
        );
        assert_eq!(
            get_game("osic").unwrap().registry_path(),
            r"SOFTWARE\Blizzard Entertainment\Battle.net\Launch Options\OSI"
        );
    }

    #[test]
    fn resolve_exe_finds_d2r_ignoring_flavor() {
        let root = temp_install("d2r", &["D2R.exe"]);
        let exe = get_game("osic").unwrap().resolve_exe(&root.to_string_lossy(), FLAVOR_RETAIL);
        assert!(exe.is_ok());
        assert!(exe.unwrap().ends_with("D2R.exe"));
    }

    #[test]
    fn resolve_exe_finds_wow_retail() {
        let root = temp_install("wow-retail", &[r"_retail_\Wow.exe"]);
        let exe = get_game("wow")
            .unwrap()
            .resolve_exe(&root.to_string_lossy(), FLAVOR_RETAIL)
            .expect("应找到正式服 exe");
        assert!(exe.ends_with(r"_retail_\Wow.exe"));
    }

    #[test]
    fn resolve_exe_finds_wow_classic_and_era() {
        let root = temp_install("wow-classic", &[r"_classic_\WowClassic.exe"]);
        let exe = get_game("wow")
            .unwrap()
            .resolve_exe(&root.to_string_lossy(), FLAVOR_CLASSIC)
            .expect("应找到怀旧服 exe");
        assert!(exe.ends_with(r"_classic_\WowClassic.exe"));

        let root = temp_install("wow-era", &[r"_classic_era_\WowClassic.exe"]);
        let exe = get_game("wow")
            .unwrap()
            .resolve_exe(&root.to_string_lossy(), FLAVOR_CLASSIC_ERA)
            .expect("应找到经典探索服 exe");
        assert!(exe.ends_with(r"_classic_era_\WowClassic.exe"));
    }

    #[test]
    fn resolve_exe_reports_missing_candidates() {
        let root = temp_install("wow-missing", &[]);
        let err = get_game("wow")
            .unwrap()
            .resolve_exe(&root.to_string_lossy(), FLAVOR_RETAIL)
            .expect_err("空目录应报错");
        assert!(err.contains("魔兽世界"), "错误信息应包含游戏名: {err}");
        assert!(err.contains("_retail_"), "错误信息应列出探测路径: {err}");
    }

    #[test]
    fn window_title_matches_any_registered_game() {
        assert!(matches_window_title("Diablo II: Resurrected"));
        assert!(matches_window_title("World of Warcraft"));
        assert!(matches_window_title("World of Warcraft Classic"));
        assert!(!matches_window_title("记事本"));
        assert!(!matches_window_title(""));
    }

    #[test]
    fn renamed_window_title_still_matches() {
        // 回归：改名后的窗口必须仍能被标题关键字识别
        let d2r = get_game("osic").unwrap().window_title_for("小号1");
        assert_eq!(d2r, "小号1 - Diablo II: Resurrected");
        assert!(matches_window_title(&d2r));

        let wow = get_game("wow").unwrap().window_title_for("小号2");
        assert_eq!(wow, "小号2 - World of Warcraft");
        assert!(matches_window_title(&wow));
    }

    #[test]
    fn installed_flavors_lists_installed_wow_branches_in_order() {
        let root = temp_install("flavors-two", &[r"_retail_\Wow.exe", r"_classic_\WowClassic.exe"]);
        let flavors = get_game("wow").unwrap().installed_flavors(&root.to_string_lossy());
        assert_eq!(flavors, vec!["retail", "classic"]);
    }

    #[test]
    fn installed_flavors_single_branch() {
        let root = temp_install("flavors-one", &[r"_classic_era_\WowClassic.exe"]);
        let flavors = get_game("wow").unwrap().installed_flavors(&root.to_string_lossy());
        assert_eq!(flavors, vec!["classic_era"]);
    }

    #[test]
    fn installed_flavors_root_exe_does_not_count() {
        // 根目录裸 WowClassic.exe 无法区分 classic/classic_era，不作为已装依据
        let root = temp_install("flavors-root", &["WowClassic.exe"]);
        assert!(get_game("wow")
            .unwrap()
            .installed_flavors(&root.to_string_lossy())
            .is_empty());
    }

    #[test]
    fn installed_flavors_empty_when_nothing_installed() {
        let root = temp_install("flavors-none", &[]);
        assert!(get_game("wow").unwrap().installed_flavors(&root.to_string_lossy()).is_empty());
    }

    #[test]
    fn resolve_exe_finds_wow_titan_and_anniversary() {
        let root = temp_install("wow-titan", &[r"_classic_titan_\WowClassic.exe"]);
        let exe = get_game("wow")
            .unwrap()
            .resolve_exe(&root.to_string_lossy(), FLAVOR_TITAN)
            .expect("应找到泰坦重铸 exe");
        assert!(exe.ends_with(r"_classic_titan_\WowClassic.exe"));

        let root = temp_install("wow-anniversary", &[r"_anniversary_\WowClassic.exe"]);
        let exe = get_game("wow")
            .unwrap()
            .resolve_exe(&root.to_string_lossy(), FLAVOR_ANNIVERSARY)
            .expect("应找到周年庆 exe");
        assert!(exe.ends_with(r"_anniversary_\WowClassic.exe"));
    }

    #[test]
    fn installed_flavors_includes_titan_and_anniversary() {
        let root = temp_install(
            "flavors-extra",
            &[
                r"_retail_\Wow.exe",
                r"_classic_titan_\WowClassic.exe",
                r"_anniversary_\WowClassic.exe",
            ],
        );
        let flavors = get_game("wow").unwrap().installed_flavors(&root.to_string_lossy());
        assert_eq!(flavors, vec!["retail", "titan", "anniversary"]);
    }

    #[test]
    fn installed_flavors_d2r_has_no_flavors() {
        // osic 无分支概念：即使 D2R.exe 存在也返回空，前端据此跳过选择弹窗
        let root = temp_install("flavors-d2r", &["D2R.exe"]);
        assert!(get_game("osic").unwrap().flavors().is_empty());
        assert!(get_game("osic").unwrap().installed_flavors(&root.to_string_lossy()).is_empty());
    }
}
