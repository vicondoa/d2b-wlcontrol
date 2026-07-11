//! Group public workload summaries into realm presentation cards.

use std::collections::HashMap;

use crate::config::UiColorArtifact;
use crate::model::{RealmGroup, RealmLauncherEntry};

const COLOR_PALETTE: &[&str] = &[
    "#7fc8ff", "#90d090", "#ffb347", "#c8a0e0", "#ff8080", "#40e0d0", "#ffd700", "#ff69b4",
    "#a0c8a0", "#d4a0ff", "#ffa07a", "#87ceeb",
];

fn color_for_name(name: &str) -> &'static str {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hash = DefaultHasher::new();
    format!("d2b-env-accent-{name}").hash(&mut hash);
    COLOR_PALETTE[(hash.finish() as usize) % COLOR_PALETTE.len()]
}

/// Build realm cards from d2b's public workload inventory.
pub fn build_realm_groups(
    workloads: Vec<RealmLauncherEntry>,
    ui_colors: Option<&UiColorArtifact>,
) -> Vec<RealmGroup> {
    let mut realm_order = Vec::<String>::new();
    let mut by_realm = HashMap::<String, (String, Vec<RealmLauncherEntry>)>::new();

    for mut workload in workloads {
        let (realm_name, realm_id) = workload_realm(&workload);
        if workload.realm_name.is_empty() {
            workload.realm_name.clone_from(&realm_name);
        }
        if workload.realm_id.is_empty() {
            workload.realm_id.clone_from(&realm_id);
        }
        let entry = by_realm.entry(realm_name.clone()).or_insert_with(|| {
            realm_order.push(realm_name);
            (realm_id, Vec::new())
        });
        entry.1.push(workload);
    }

    realm_order
        .into_iter()
        .filter_map(|realm_name| {
            let (realm_id, workloads) = by_realm.remove(&realm_name)?;
            Some(RealmGroup {
                realm_color: resolve_realm_color(&realm_name, ui_colors),
                realm_name,
                realm_id,
                workloads,
            })
        })
        .collect()
}

fn workload_realm(workload: &RealmLauncherEntry) -> (String, String) {
    if !workload.realm_name.is_empty() {
        let realm_id = if workload.realm_id.is_empty() {
            workload.realm_name.clone()
        } else {
            workload.realm_id.clone()
        };
        return (workload.realm_name.clone(), realm_id);
    }

    let labels = workload
        .canonical_target
        .strip_suffix(".d2b")
        .unwrap_or(&workload.canonical_target)
        .split('.')
        .collect::<Vec<_>>();
    let realm = labels.get(1).copied().unwrap_or("default").to_owned();
    (realm.clone(), realm)
}

fn resolve_realm_color(realm_name: &str, ui_colors: Option<&UiColorArtifact>) -> String {
    if let Some(colors) = ui_colors {
        if let Some(realm_color) = colors.realms.get(realm_name) {
            return realm_color.accent.clone();
        }
        if let Some(env_color) = colors.envs.get(realm_name) {
            return env_color.accent.clone();
        }
    }
    color_for_name(realm_name).to_owned()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::config::{UiColorEnv, UiColorHost, UiColorRealm, UiColorStates};
    use crate::model::{IsolationPosture, WorkloadExecutionPosture, WorkloadProviderKind};

    fn workload(
        realm: &str,
        name: &str,
        provider_kind: WorkloadProviderKind,
    ) -> RealmLauncherEntry {
        RealmLauncherEntry {
            workload_name: name.to_owned(),
            label: name.to_owned(),
            canonical_target: format!("{name}.{realm}.d2b"),
            realm_name: realm.to_owned(),
            realm_id: realm.to_owned(),
            provider_kind,
            execution_posture: WorkloadExecutionPosture {
                isolation: if provider_kind == WorkloadProviderKind::UnsafeLocal {
                    IsolationPosture::UnsafeLocal
                } else {
                    IsolationPosture::VirtualMachine
                },
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn ui_colors(realm: &str, realm_accent: Option<&str>, env_accent: &str) -> UiColorArtifact {
        let mut envs = BTreeMap::new();
        envs.insert(
            realm.to_owned(),
            UiColorEnv {
                accent: env_accent.to_owned(),
            },
        );
        let mut realms = BTreeMap::new();
        if let Some(accent) = realm_accent {
            realms.insert(
                realm.to_owned(),
                UiColorRealm {
                    accent: accent.to_owned(),
                    path: realm.to_owned(),
                },
            );
        }
        UiColorArtifact {
            version: 1,
            host: UiColorHost {
                accent: "#7fc8ff".to_owned(),
            },
            states: UiColorStates {
                running: "#a6e3a1".to_owned(),
                transitioning: "#f9e2af".to_owned(),
                pending_restart: "#fab387".to_owned(),
                error: "#f38ba8".to_owned(),
                denied: "#cba6f7".to_owned(),
                unknown: "#6c7086".to_owned(),
            },
            envs,
            realms,
            vms: BTreeMap::new(),
        }
    }

    #[test]
    fn groups_public_workloads_and_preserves_order() {
        let groups = build_realm_groups(
            vec![
                workload("personal", "browser", WorkloadProviderKind::LocalVm),
                workload("work", "builder", WorkloadProviderKind::LocalVm),
                workload("personal", "terminal", WorkloadProviderKind::UnsafeLocal),
            ],
            None,
        );
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].realm_name, "personal");
        assert_eq!(groups[0].workloads.len(), 2);
        assert_eq!(groups[1].realm_name, "work");
    }

    #[test]
    fn prefers_realm_then_environment_accent() {
        let workload = workload("work", "browser", WorkloadProviderKind::LocalVm);
        let realm = ui_colors("work", Some("#ffb347"), "#ffa500");
        assert_eq!(
            build_realm_groups(vec![workload.clone()], Some(&realm))[0].realm_color,
            "#ffb347"
        );
        let environment = ui_colors("work", None, "#ffa500");
        assert_eq!(
            build_realm_groups(vec![workload], Some(&environment))[0].realm_color,
            "#ffa500"
        );
    }

    #[test]
    fn identifies_all_unsafe_and_mixed_cards() {
        let all_unsafe = build_realm_groups(
            vec![
                workload("host", "apps", WorkloadProviderKind::UnsafeLocal),
                workload("host", "terminal", WorkloadProviderKind::UnsafeLocal),
            ],
            None,
        );
        assert!(all_unsafe[0].all_unsafe_local());
        assert!(!all_unsafe[0].has_mixed_isolation());

        let mixed = build_realm_groups(
            vec![
                workload("work", "apps", WorkloadProviderKind::UnsafeLocal),
                workload("work", "builder", WorkloadProviderKind::LocalVm),
            ],
            None,
        );
        assert!(!mixed[0].all_unsafe_local());
        assert!(mixed[0].has_mixed_isolation());
    }
}
