//! Project navigation menus and asynchronous refresh of the active server.
use crate::*;

pub fn install(ui: &UiRc, arquivo: &gio::Menu) {
    let local = gio::Menu::new();
    local.append(Some("Abrir projeto…"), Some("win.pasta"));
    local.append(Some("Projetos recentes"), Some("win.recentes"));
    arquivo.append_submenu(Some("Local"), &local);
    arquivo.append_submenu(Some("Remoto"), &ui.borrow().remote_menu);
    let action = gio::SimpleAction::new_stateful("projetos-abertos", None, &false.to_variant());
    let weak = Rc::downgrade(ui);
    action.connect_change_state(move |a, state| {
        if let Some(state) = state {
            a.set_state(state);
            if state.get::<bool>() == Some(true)
                && let Some(ui) = weak.upgrade()
            {
                refresh(&ui);
            }
        }
    });
    ui.borrow().window.add_action(&action);
    let action = gio::SimpleAction::new("abrir-projeto-remoto", Some(glib::VariantTy::STRING));
    let weak = Rc::downgrade(ui);
    action.connect_activate(move |_, value| {
        let Some(ui) = weak.upgrade() else { return };
        let Some(projeto) = value.and_then(|v| v.str()).map(str::to_string) else {
            return;
        };
        let connection = {
            let u = ui.borrow();
            match &u.files {
                Files::Remote { rpc, .. } => {
                    Some((Arc::clone(rpc), u.prefs.servidor.clone(), u.project.clone()))
                }
                _ => None,
            }
        };
        if let Some((rpc, url, aluno)) = connection {
            let next = Rc::clone(&ui);
            after_unsaved(&ui, move || apply_remote(&next, rpc, url, aluno, projeto));
        }
    });
    ui.borrow().window.add_action(&action);
    rebuild(ui);
}

pub fn rebuild(ui: &UiRc) {
    let u = ui.borrow();
    populate(
        &u.remote_menu,
        &u.projects_menu,
        u.files
            .is_remote()
            .then_some((u.prefs.servidor.as_str(), u.project.as_str())),
    );
}

fn populate(remote: &gio::Menu, projects: &gio::Menu, connection: Option<(&str, &str)>) {
    remote.remove_all();
    projects.remove_all();
    if let Some((url, aluno)) = connection {
        let item = gio::MenuItem::new_submenu(
            Some(&format!("Projetos do servidor conectado — {url} — {aluno}")),
            projects,
        );
        item.set_attribute_value("submenu-action", Some(&"win.projetos-abertos".to_variant()));
        remote.append_item(&item);
        remote.append(Some("Novo projeto…"), Some("win.novo-projeto-remoto"));
        remote.append(Some("Sincronização…"), Some("win.sincronizacao"));
    }
    remote.append(Some("Conectar ao servidor…"), Some("win.servidor"));
    remote.append(
        Some("Servidores anteriores"),
        Some("win.servidores-anteriores"),
    );
}

pub fn refresh(ui: &UiRc) {
    ui.borrow_mut().listing_epoch += 1;
    let listing_epoch = ui.borrow().listing_epoch;
    let (rpc, aluno, atual, menu) = {
        let u = ui.borrow();
        let Files::Remote { rpc, projeto } = &u.files else {
            return;
        };
        (
            Arc::clone(rpc),
            u.project.clone(),
            projeto.clone(),
            u.projects_menu.clone(),
        )
    };
    navigation_task(
        ui,
        move || rpc.list_projects(&aluno),
        move |ui, projetos| {
            if ui.borrow().listing_epoch != listing_epoch {
                return;
            }
            fill_projects(&menu, projetos, &atual);
        },
    );
}

fn fill_projects(menu: &gio::Menu, projetos: Vec<String>, atual: &str) {
    menu.remove_all();
    for projeto in projetos {
        let label = if projeto == atual {
            format!("✓ {projeto}")
        } else {
            projeto.clone()
        };
        let item = gio::MenuItem::new(Some(&label), None);
        item.set_action_and_target_value(
            Some("win.abrir-projeto-remoto"),
            Some(&projeto.to_variant()),
        );
        menu.append_item(&item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn labels(menu: &gio::Menu) -> Vec<String> {
        (0..menu.n_items())
            .map(|i| {
                menu.item_attribute_value(i, "label", None)
                    .unwrap()
                    .str()
                    .unwrap()
                    .to_string()
            })
            .collect()
    }
    #[test]
    fn connection_actions_only_exist_for_active_remote_project() {
        let remote = gio::Menu::new();
        let projects = gio::Menu::new();
        for _ in 0..2 {
            // no project and local project have no active connection
            populate(&remote, &projects, None);
            assert_eq!(
                labels(&remote),
                ["Conectar ao servidor…", "Servidores anteriores"]
            );
        }
        populate(&remote, &projects, Some(("http://server", "ana")));
        assert_eq!(remote.n_items(), 5);
        assert!(labels(&remote)[0].contains("http://server — ana"));
        assert_eq!(remote.item_link(0, "submenu").unwrap(), projects);
        populate(&remote, &projects, None);
        assert_eq!(remote.n_items(), 2);
    }
    #[test]
    fn refresh_replaces_list_and_marks_current_project() {
        let projects = gio::Menu::new();
        fill_projects(&projects, vec!["a".into(), "b".into()], "b");
        assert_eq!(labels(&projects), ["a", "✓ b"]);
        assert_eq!(
            projects
                .item_attribute_value(1, "target", None)
                .unwrap()
                .str(),
            Some("b")
        );
        fill_projects(&projects, vec!["new".into()], "new");
        assert_eq!(labels(&projects), ["✓ new"]);
    }
}
