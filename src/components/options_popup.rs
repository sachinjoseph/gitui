use std::{cell::RefCell, rc::Rc};

use super::{
	visibility_blocking, CommandBlocking, CommandInfo, Component,
	DrawableComponent, EventState,
};
use crate::{
	components::utils::string_width_align,
	keys::{key_match, SharedKeyConfig},
	queue::{InternalEvent, Queue},
	strings::{self},
	ui::{self, style::SharedTheme},
};
use anyhow::Result;
use asyncgit::sync::{diff::DiffOptions, ShowUntrackedFilesConfig};
use crossterm::event::Event;
use tui::{
	backend::Backend,
	layout::{Alignment, Rect},
	style::{Modifier, Style},
	text::{Span, Spans},
	widgets::{Block, Borders, Clear, Paragraph},
	Frame,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AppOption {
	StatusShowUntracked,
	DiffIgnoreWhitespaces,
	DiffContextLines,
	DiffInterhunkLines,
}

#[derive(Default, Copy, Clone)]
pub struct Options {
	pub status_show_untracked: Option<ShowUntrackedFilesConfig>,
	pub diff: DiffOptions,
}

pub type SharedOptions = Rc<RefCell<Options>>;

pub struct OptionsPopupComponent {
	selection: AppOption,
	queue: Queue,
	visible: bool,
	key_config: SharedKeyConfig,
	options: SharedOptions,
	theme: SharedTheme,
}

impl OptionsPopupComponent {
	///
	pub fn new(
		queue: &Queue,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
		options: SharedOptions,
	) -> Self {
		Self {
			selection: AppOption::StatusShowUntracked,
			queue: queue.clone(),
			visible: false,
			key_config,
			options,
			theme,
		}
	}

	fn get_text(&self, width: u16) -> Vec<Spans> {
		let mut txt: Vec<Spans> = Vec::with_capacity(10);

		self.add_status(&mut txt, width);

		txt
	}

	fn add_status(&self, txt: &mut Vec<Spans>, width: u16) {
		Self::add_header(txt, "Status");

		self.add_entry(
			txt,
			width,
			"Show untracked",
			match self.options.borrow().status_show_untracked {
				None => "Gitconfig",
				Some(ShowUntrackedFilesConfig::No) => "No",
				Some(ShowUntrackedFilesConfig::Normal) => "Normal",
				Some(ShowUntrackedFilesConfig::All) => "All",
			},
			self.is_select(AppOption::StatusShowUntracked),
		);
		Self::add_header(txt, "");

		Self::add_header(txt, "Diff");
		self.add_entry(
			txt,
			width,
			"Ignore whitespaces",
			&self.options.borrow().diff.ignore_whitespace.to_string(),
			self.is_select(AppOption::DiffIgnoreWhitespaces),
		);
		self.add_entry(
			txt,
			width,
			"Context lines",
			&self.options.borrow().diff.context.to_string(),
			self.is_select(AppOption::DiffContextLines),
		);
		self.add_entry(
			txt,
			width,
			"Inter hunk lines",
			&self.options.borrow().diff.interhunk_lines.to_string(),
			self.is_select(AppOption::DiffInterhunkLines),
		);
	}

	fn is_select(&self, kind: AppOption) -> bool {
		self.selection == kind
	}

	fn add_header(txt: &mut Vec<Spans>, header: &'static str) {
		txt.push(Spans::from(vec![Span::styled(
			header,
			//TODO: use style
			Style::default().add_modifier(Modifier::UNDERLINED),
		)]));
	}

	fn add_entry(
		&self,
		txt: &mut Vec<Spans>,
		width: u16,
		entry: &'static str,
		value: &str,
		selected: bool,
	) {
		let half = usize::from(width / 2);
		txt.push(Spans::from(vec![
			Span::styled(
				string_width_align(entry, half),
				self.theme.text(true, false),
			),
			Span::styled(
				format!("{:^w$}", value, w = half),
				self.theme.text(true, selected),
			),
		]));
	}

	fn move_selection(&mut self, up: bool) {
		if up {
			self.selection = match self.selection {
				AppOption::StatusShowUntracked => {
					AppOption::DiffInterhunkLines
				}
				AppOption::DiffIgnoreWhitespaces => {
					AppOption::StatusShowUntracked
				}
				AppOption::DiffContextLines => {
					AppOption::DiffIgnoreWhitespaces
				}
				AppOption::DiffInterhunkLines => {
					AppOption::DiffContextLines
				}
			};
		} else {
			self.selection = match self.selection {
				AppOption::StatusShowUntracked => {
					AppOption::DiffIgnoreWhitespaces
				}
				AppOption::DiffIgnoreWhitespaces => {
					AppOption::DiffContextLines
				}
				AppOption::DiffContextLines => {
					AppOption::DiffInterhunkLines
				}
				AppOption::DiffInterhunkLines => {
					AppOption::StatusShowUntracked
				}
			};
		}
	}

	fn switch_option(&mut self, right: bool) {
		if right {
			match self.selection {
				AppOption::StatusShowUntracked => {
					let untracked =
						self.options.borrow().status_show_untracked;

					let untracked = match untracked {
						None => {
							Some(ShowUntrackedFilesConfig::Normal)
						}
						Some(ShowUntrackedFilesConfig::Normal) => {
							Some(ShowUntrackedFilesConfig::All)
						}
						Some(ShowUntrackedFilesConfig::All) => {
							Some(ShowUntrackedFilesConfig::No)
						}
						Some(ShowUntrackedFilesConfig::No) => None,
					};

					self.options.borrow_mut().status_show_untracked =
						untracked;
				}
				AppOption::DiffIgnoreWhitespaces => {
					let old =
						self.options.borrow().diff.ignore_whitespace;
					self.options
						.borrow_mut()
						.diff
						.ignore_whitespace = !old;
				}
				AppOption::DiffContextLines => {
					let old = self.options.borrow().diff.context;
					self.options.borrow_mut().diff.context =
						old.saturating_add(1);
				}
				AppOption::DiffInterhunkLines => {
					let old =
						self.options.borrow().diff.interhunk_lines;
					self.options.borrow_mut().diff.interhunk_lines =
						old.saturating_add(1);
				}
			};
		} else {
			match self.selection {
				AppOption::StatusShowUntracked => {
					let untracked =
						self.options.borrow().status_show_untracked;

					let untracked = match untracked {
						None => Some(ShowUntrackedFilesConfig::No),
						Some(ShowUntrackedFilesConfig::No) => {
							Some(ShowUntrackedFilesConfig::All)
						}
						Some(ShowUntrackedFilesConfig::All) => {
							Some(ShowUntrackedFilesConfig::Normal)
						}
						Some(ShowUntrackedFilesConfig::Normal) => {
							None
						}
					};

					self.options.borrow_mut().status_show_untracked =
						untracked;
				}
				AppOption::DiffIgnoreWhitespaces => {
					let old =
						self.options.borrow().diff.ignore_whitespace;
					self.options
						.borrow_mut()
						.diff
						.ignore_whitespace = !old;
				}
				AppOption::DiffContextLines => {
					let old = self.options.borrow().diff.context;
					self.options.borrow_mut().diff.context =
						old.saturating_sub(1);
				}
				AppOption::DiffInterhunkLines => {
					let old =
						self.options.borrow().diff.interhunk_lines;
					self.options.borrow_mut().diff.interhunk_lines =
						old.saturating_sub(1);
				}
			};
		}

		self.queue
			.push(InternalEvent::OptionSwitched(self.selection));
	}
}

impl DrawableComponent for OptionsPopupComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		area: Rect,
	) -> Result<()> {
		if self.is_visible() {
			const SIZE: (u16, u16) = (50, 10);
			let area =
				ui::centered_rect_absolute(SIZE.0, SIZE.1, area);

			let width = area.width;

			f.render_widget(Clear, area);
			f.render_widget(
				Paragraph::new(self.get_text(width))
					.block(
						Block::default()
							.borders(Borders::ALL)
							.title(Span::styled(
								"Options",
								self.theme.title(true),
							))
							.border_style(self.theme.block(true)),
					)
					.alignment(Alignment::Left),
				area,
			);
		}

		Ok(())
	}
}

impl Component for OptionsPopupComponent {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.is_visible() || force_all {
			out.push(
				CommandInfo::new(
					strings::commands::close_popup(&self.key_config),
					true,
					true,
				)
				.order(1),
			);
			out.push(
				CommandInfo::new(
					strings::commands::navigate_tree(
						&self.key_config,
					),
					true,
					true,
				)
				.order(1),
			);
		}

		visibility_blocking(self)
	}

	fn event(
		&mut self,
		event: &crossterm::event::Event,
	) -> Result<EventState> {
		if self.is_visible() {
			if let Event::Key(key) = &event {
				if key_match(key, self.key_config.keys.exit_popup) {
					self.hide();
				} else if key_match(key, self.key_config.keys.move_up)
				{
					self.move_selection(true);
				} else if key_match(
					key,
					self.key_config.keys.move_down,
				) {
					self.move_selection(false);
				} else if key_match(
					key,
					self.key_config.keys.move_right,
				) {
					self.switch_option(true);
				} else if key_match(
					key,
					self.key_config.keys.move_left,
				) {
					self.switch_option(false);
				}
			}

			return Ok(EventState::Consumed);
		}

		Ok(EventState::NotConsumed)
	}

	fn is_visible(&self) -> bool {
		self.visible
	}

	fn hide(&mut self) {
		self.visible = false;
	}

	fn show(&mut self) -> Result<()> {
		self.visible = true;

		Ok(())
	}
}
