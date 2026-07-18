use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures_util::StreamExt;
use ratatui::{Terminal, backend::Backend, widgets::TableState};

use crate::{
    api::HereNowClient,
    model::{Analytics, Drive, DriveFile, Profile, Site},
    ui,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Section {
    #[default]
    Sites,
    Drives,
    Account,
}

impl Section {
    pub const ALL: [Self; 3] = [Self::Sites, Self::Drives, Self::Account];

    pub fn label(self) -> &'static str {
        match self {
            Self::Sites => "Sites",
            Self::Drives => "Drives",
            Self::Account => "Account",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Prompt {
    Search,
    EditName,
    ConfirmDelete { slug: String },
}

pub struct App {
    pub client: HereNowClient,
    pub section: Section,
    pub sites: Vec<Site>,
    pub site_state: TableState,
    pub site_detail: Option<Site>,
    pub analytics: Option<Analytics>,
    pub drives: Vec<Drive>,
    pub drive_state: TableState,
    pub drive_files: Vec<DriveFile>,
    pub profile: Option<Profile>,
    pub prompt: Option<Prompt>,
    pub input: String,
    pub status: String,
    pub error: Option<String>,
    pub show_help: bool,
    should_quit: bool,
}

impl App {
    pub fn new(client: HereNowClient) -> Self {
        let mut site_state = TableState::default();
        site_state.select(Some(0));
        let mut drive_state = TableState::default();
        drive_state.select(Some(0));
        Self {
            client,
            section: Section::Sites,
            sites: Vec::new(),
            site_state,
            site_detail: None,
            analytics: None,
            drives: Vec::new(),
            drive_state,
            drive_files: Vec::new(),
            profile: None,
            prompt: None,
            input: String::new(),
            status: "Connecting…".into(),
            error: None,
            show_help: false,
            should_quit: false,
        }
    }

    pub async fn run<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        tick_rate: Duration,
    ) -> Result<()>
    where
        B::Error: Send + Sync + 'static,
    {
        let mut events = EventStream::new();
        let mut tick = tokio::time::interval(tick_rate);
        loop {
            terminal.draw(|frame| ui::draw(frame, self))?;
            tokio::select! {
                _ = tick.tick() => {}
                maybe_event = events.next() => {
                    if let Some(Ok(Event::Key(key))) = maybe_event
                        && key.kind == KeyEventKind::Press
                    {
                        self.handle_key(key).await;
                    }
                }
            }
            if self.should_quit {
                return Ok(());
            }
        }
    }

    pub async fn refresh(&mut self) {
        self.status = "Refreshing…".into();
        self.error = None;
        let (sites, drives, profile) = tokio::join!(
            self.client.sites(),
            self.client.drives(),
            self.client.profile()
        );
        match sites {
            Ok(sites) => {
                self.sites = sites;
                self.clamp_site_selection();
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        match drives {
            Ok(response) => {
                self.drives = response.drives;
                self.clamp_drive_selection();
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        match profile {
            Ok(profile) => self.profile = Some(profile),
            Err(error) => self.error = Some(error.to_string()),
        }
        self.status = format!("{} Sites · {} Drives", self.sites.len(), self.drives.len());
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        if self.show_help {
            self.show_help = false;
            return;
        }
        if self.prompt.is_some() {
            self.handle_prompt_key(key).await;
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true
            }
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Tab | KeyCode::Char('l') | KeyCode::Right => self.next_section(),
            KeyCode::BackTab | KeyCode::Char('h') | KeyCode::Left => self.previous_section(),
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Char('g') | KeyCode::Home => self.select_edge(false),
            KeyCode::Char('G') | KeyCode::End => self.select_edge(true),
            KeyCode::Char('r') => self.refresh().await,
            KeyCode::Char('/') if self.section == Section::Sites => {
                self.prompt = Some(Prompt::Search);
                self.input.clear();
            }
            KeyCode::Enter => self.inspect_selected().await,
            KeyCode::Char('o') => self.open_selected(),
            KeyCode::Char('e') if self.section == Section::Sites => self.begin_edit(),
            KeyCode::Char('d') if self.section == Section::Sites => self.duplicate_selected().await,
            KeyCode::Char('x') if self.section == Section::Sites => self.begin_delete(),
            KeyCode::Char('a') if self.section == Section::Sites => self.load_analytics().await,
            _ => {}
        }
    }

    async fn handle_prompt_key(&mut self, key: KeyEvent) {
        let prompt = self.prompt.clone().expect("prompt exists");
        if let Prompt::ConfirmDelete { slug } = prompt {
            match key.code {
                KeyCode::Char('y') => {
                    self.prompt = None;
                    self.status = format!("Deleting {slug}…");
                    match self.client.delete_site(&slug).await {
                        Ok(()) => {
                            self.status = format!("Deleted {slug}");
                            self.site_detail = None;
                            self.analytics = None;
                            self.refresh().await;
                        }
                        Err(error) => self.error = Some(error.to_string()),
                    }
                }
                KeyCode::Esc | KeyCode::Char('n') => self.prompt = None,
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.prompt = None;
                self.input.clear();
            }
            KeyCode::Enter => {
                let input = self.input.trim().to_owned();
                self.prompt = None;
                self.input.clear();
                if input.is_empty() {
                    return;
                }
                match prompt {
                    Prompt::Search => self.search(&input).await,
                    Prompt::EditName => self.save_name(&input).await,
                    Prompt::ConfirmDelete { .. } => unreachable!(),
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(character)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.input.push(character);
            }
            _ => {}
        }
    }

    async fn search(&mut self, query: &str) {
        self.status = format!("Searching for {query}…");
        match self.client.search(query).await {
            Ok(sites) => {
                self.sites = sites;
                self.site_state
                    .select((!self.sites.is_empty()).then_some(0));
                self.site_detail = None;
                self.analytics = None;
                self.status = format!("{} matches for “{query}” · r clears", self.sites.len());
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    async fn save_name(&mut self, name: &str) {
        let Some(slug) = self.selected_site().map(|site| site.slug.clone()) else {
            return;
        };
        self.status = format!("Updating {slug}…");
        match self.client.patch_metadata(&slug, name, None).await {
            Ok(()) => {
                self.status = format!("Renamed {slug}");
                self.refresh().await;
                self.load_site_detail(slug).await;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    async fn inspect_selected(&mut self) {
        match self.section {
            Section::Sites => {
                if let Some(slug) = self.selected_site().map(|site| site.slug.clone()) {
                    self.load_site_detail(slug).await;
                }
            }
            Section::Drives => {
                if let Some(drive) = self.selected_drive().cloned() {
                    self.status = format!("Loading {}…", drive.name);
                    match self.client.drive_files(&drive.id).await {
                        Ok(files) => {
                            self.drive_files = files;
                            self.status =
                                format!("{} · {} files", drive.name, self.drive_files.len());
                        }
                        Err(error) => self.error = Some(error.to_string()),
                    }
                }
            }
            Section::Account => {
                if let Some(profile) = &self.profile {
                    let _ = open::that(&profile.url);
                }
            }
        }
    }

    async fn load_site_detail(&mut self, slug: String) {
        self.status = format!("Loading {slug}…");
        match self.client.site(&slug).await {
            Ok(site) => {
                self.status = format!("{} files · Enter refreshes detail", site.manifest.len());
                self.site_detail = Some(site);
                self.analytics = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    async fn duplicate_selected(&mut self) {
        let Some(slug) = self.selected_site().map(|site| site.slug.clone()) else {
            return;
        };
        self.status = format!("Duplicating {slug}…");
        match self.client.duplicate(&slug).await {
            Ok(site) => {
                self.status = format!("Duplicated as {}", site.slug);
                self.refresh().await;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    async fn load_analytics(&mut self) {
        let Some(slug) = self.selected_site().map(|site| site.slug.clone()) else {
            return;
        };
        self.status = format!("Loading 30-day analytics for {slug}…");
        match self.client.analytics(&slug, "30d").await {
            Ok(analytics) => {
                self.analytics = Some(analytics);
                self.status = "30-day analytics loaded".into();
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn begin_edit(&mut self) {
        if let Some(site) = self.selected_site() {
            self.input = site.label().to_owned();
            self.prompt = Some(Prompt::EditName);
        }
    }

    fn begin_delete(&mut self) {
        if let Some(site) = self.selected_site() {
            self.prompt = Some(Prompt::ConfirmDelete {
                slug: site.slug.clone(),
            });
        }
    }

    fn open_selected(&mut self) {
        let target = match self.section {
            Section::Sites => self.selected_site().map(|site| site.site_url.clone()),
            Section::Drives => self
                .selected_drive()
                .and_then(|drive| drive.dashboard_url.clone()),
            Section::Account => self.profile.as_ref().map(|profile| profile.url.clone()),
        };
        if let Some(target) = target {
            match open::that(&target) {
                Ok(()) => self.status = format!("Opened {target}"),
                Err(error) => self.error = Some(format!("could not open URL: {error}")),
            }
        }
    }

    fn next_section(&mut self) {
        self.section = match self.section {
            Section::Sites => Section::Drives,
            Section::Drives => Section::Account,
            Section::Account => Section::Sites,
        };
    }

    fn previous_section(&mut self) {
        self.section = match self.section {
            Section::Sites => Section::Account,
            Section::Drives => Section::Sites,
            Section::Account => Section::Drives,
        };
    }

    fn move_selection(&mut self, delta: isize) {
        let (state, length) = match self.section {
            Section::Sites => (&mut self.site_state, self.sites.len()),
            Section::Drives => (&mut self.drive_state, self.drives.len()),
            Section::Account => return,
        };
        if length == 0 {
            state.select(None);
            return;
        }
        let current = state.selected().unwrap_or_default() as isize;
        let next = (current + delta).clamp(0, length.saturating_sub(1) as isize) as usize;
        state.select(Some(next));
    }

    fn select_edge(&mut self, end: bool) {
        let (state, length) = match self.section {
            Section::Sites => (&mut self.site_state, self.sites.len()),
            Section::Drives => (&mut self.drive_state, self.drives.len()),
            Section::Account => return,
        };
        state.select((length > 0).then_some(if end { length - 1 } else { 0 }));
    }

    fn clamp_site_selection(&mut self) {
        let selected = self
            .site_state
            .selected()
            .unwrap_or_default()
            .min(self.sites.len().saturating_sub(1));
        self.site_state
            .select((!self.sites.is_empty()).then_some(selected));
    }

    fn clamp_drive_selection(&mut self) {
        let selected = self
            .drive_state
            .selected()
            .unwrap_or_default()
            .min(self.drives.len().saturating_sub(1));
        self.drive_state
            .select((!self.drives.is_empty()).then_some(selected));
    }

    pub fn selected_site(&self) -> Option<&Site> {
        self.site_state
            .selected()
            .and_then(|index| self.sites.get(index))
    }

    pub fn selected_drive(&self) -> Option<&Drive> {
        self.drive_state
            .selected()
            .and_then(|index| self.drives.get(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_navigation_wraps() {
        let mut section = Section::Sites;
        section = match section {
            Section::Sites => Section::Drives,
            _ => unreachable!(),
        };
        assert_eq!(section, Section::Drives);
        assert_eq!(Section::ALL.len(), 3);
    }
}
