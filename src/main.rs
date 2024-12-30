use crossterm::{
    cursor,
    event::read,
    execute,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};

const MAX_HISTORY: usize = 50;
const BOOKMARKS_FILE: &str = "bookmarks.json";

#[derive(Debug, Serialize, Deserialize)]
struct Bookmark {
    title: String,
    url: String,
}

struct Browser {
    client: Client,
    current_url: Option<String>,
    history: VecDeque<String>,
    bookmarks: Vec<Bookmark>,
    page_content: String,
    scroll_position: usize,
}

impl Browser {
    fn new() -> Self {
        Browser {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap(),
            current_url: None,
            history: VecDeque::with_capacity(MAX_HISTORY),
            bookmarks: Self::load_bookmarks(),
            page_content: String::new(),
            scroll_position: 0,
        }
    }

    fn load_bookmarks() -> Vec<Bookmark> {
        if let Ok(file) = File::open(BOOKMARKS_FILE) {
            serde_json::from_reader(file).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    fn save_bookmarks(&self) -> io::Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(BOOKMARKS_FILE)?;
        serde_json::to_writer_pretty(file, &self.bookmarks)?;
        Ok(())
    }

    fn add_to_history(&mut self, url: String) {
        if let Some(pos) = self.history.iter().position(|x| x == &url) {
            self.history.remove(pos);
        }

        if self.history.len() >= MAX_HISTORY {
            self.history.pop_back();
        }
        self.history.push_front(url);
    }

    fn navigate(&mut self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        let url = if !url.starts_with("http") {
            format!("https://{}", url)
        } else {
            url.to_string()
        };

        let response = self.client.get(&url).send()?;
        let url_clone = url.clone();
        self.handle_response(response, &url_clone)?;
        self.add_to_history(url.clone());
        self.current_url = Some(url);
        self.scroll_position = 0;
        Ok(())
    }

    fn handle_response(
        &mut self,
        response: Response,
        _url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.contains("text/html") {
            let text = response.text()?;
            self.page_content = self.render_html(&text);
        } else if content_type.contains("application/json") {
            let json: serde_json::Value = response.json()?;
            self.page_content = serde_json::to_string_pretty(&json)?;
        } else {
            self.page_content =
                format!("Content-Type '{}' not supported for display", content_type);
        }

        self.display_page()?;
        Ok(())
    }

    fn render_html(&self, html: &str) -> String {
        html2text::from_read(html.as_bytes(), 100)
    }

    fn display_page(&self) -> io::Result<()> {
        execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0))?;

        execute!(
            io::stdout(),
            SetBackgroundColor(Color::Blue),
            SetForegroundColor(Color::White)
        )?;

        let terminal_width = crossterm::terminal::size()?.0 as usize;
        let header = format!(" Rust Web Browser ");
        let padding = " ".repeat(terminal_width - header.len());
        println!("{}{}", header, padding);

        execute!(
            io::stdout(),
            ResetColor,
            SetBackgroundColor(Color::Black),
            SetForegroundColor(Color::Green)
        )?;

        let url = self.current_url.as_deref().unwrap_or("No URL");
        println!("└─ URL: {}\n", url);

        execute!(io::stdout(), ResetColor)?;

        let lines: Vec<&str> = self.page_content.lines().collect();
        let terminal_height = crossterm::terminal::size()?.1 as usize - 7;

        let max_scroll = if lines.len() > terminal_height {
            lines.len() - terminal_height
        } else {
            0
        };

        let effective_scroll = std::cmp::min(self.scroll_position, max_scroll);
        let visible_lines = &lines[effective_scroll..];

        for (i, line) in visible_lines.iter().enumerate().take(terminal_height) {
            if line.trim().starts_with('#') {
                execute!(io::stdout(), SetForegroundColor(Color::Cyan))?;
                println!("{:4} │ {}", i + effective_scroll + 1, line);
                execute!(io::stdout(), ResetColor)?;
            } else if line.contains("http") || line.contains("www.") {
                execute!(io::stdout(), SetForegroundColor(Color::Blue))?;
                println!("{:4} │ {}", i + effective_scroll + 1, line);
                execute!(io::stdout(), ResetColor)?;
            } else {
                execute!(io::stdout(), SetForegroundColor(Color::White))?;
                println!("{:4} │ {}", i + effective_scroll + 1, line);
            }
        }

        execute!(
            io::stdout(),
            cursor::MoveTo(0, (terminal_height + 5) as u16),
            SetBackgroundColor(Color::DarkGrey),
            SetForegroundColor(Color::White)
        )?;

        let status = format!(
            " Lines: {} | Position: {} ",
            lines.len(),
            effective_scroll + 1
        );
        let status_padding = " ".repeat(terminal_width - status.len());
        println!("{}{}", status, status_padding);

        execute!(
            io::stdout(),
            ResetColor,
            SetForegroundColor(Color::DarkGrey)
        )?;
        println!("\n[Press 'h' for help] [w/s to scroll] [q to quit]");

        io::stdout().flush()?;
        Ok(())
    }

    fn add_bookmark(&mut self, title: &str) -> io::Result<()> {
        if let Some(url) = &self.current_url {
            self.bookmarks.push(Bookmark {
                title: title.to_string(),
                url: url.clone(),
            });
            self.save_bookmarks()?;
        }
        Ok(())
    }

    fn show_bookmarks(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            execute!(io::stdout(), Clear(ClearType::All))?;

            execute!(
                io::stdout(),
                SetBackgroundColor(Color::Magenta),
                SetForegroundColor(Color::White)
            )?;
            println!(" Bookmarks ");
            execute!(io::stdout(), ResetColor)?;
            println!();

            for (i, bookmark) in self.bookmarks.iter().enumerate() {
                execute!(io::stdout(), SetForegroundColor(Color::Yellow))?;
                print!(" {}. ", i + 1);

                execute!(io::stdout(), SetForegroundColor(Color::White))?;
                print!("{} ", bookmark.title);

                execute!(io::stdout(), SetForegroundColor(Color::Blue))?;
                println!("({})", bookmark.url);
            }

            execute!(io::stdout(), ResetColor)?;
            println!("\nCommands:");
            println!("number - Go to bookmark");
            println!("d number - Delete bookmark");
            println!("q - Return to browser");

            print!("\nEnter command: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input == "q" {
                break;
            } else if input.starts_with('d') {
                if let Some(num) = input.split_whitespace().nth(1) {
                    if let Ok(index) = num.parse::<usize>() {
                        if index > 0 && index <= self.bookmarks.len() {
                            self.bookmarks.remove(index - 1);
                            self.save_bookmarks()?;
                            println!("Bookmark deleted!");
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                    }
                }
            } else if let Ok(index) = input.parse::<usize>() {
                if index > 0 && index <= self.bookmarks.len() {
                    let url = self.bookmarks[index - 1].url.clone();
                    self.navigate(&url)?;
                    break;
                }
            }
        }
        Ok(())
    }

    fn show_history(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            execute!(io::stdout(), Clear(ClearType::All))?;

            execute!(
                io::stdout(),
                SetBackgroundColor(Color::DarkBlue),
                SetForegroundColor(Color::White)
            )?;
            println!(" Browsing History ");
            execute!(io::stdout(), ResetColor)?;
            println!();

            for (i, url) in self.history.iter().enumerate() {
                execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                print!(" {}. ", i + 1);

                execute!(io::stdout(), SetForegroundColor(Color::Blue))?;
                println!("{}", url);
            }

            execute!(io::stdout(), ResetColor)?;
            println!("\nCommands:");
            println!("number - Go to URL from history");
            println!("q - Return to browser");

            print!("\nEnter command: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input == "q" {
                break;
            } else if let Ok(index) = input.parse::<usize>() {
                if index > 0 && index <= self.history.len() {
                    let url = self.history[index - 1].clone();
                    self.navigate(&url)?;
                    break;
                }
            }
        }
        Ok(())
    }

    fn view_page_source(&self) -> io::Result<()> {
        execute!(io::stdout(), Clear(ClearType::All))?;
        println!("Page Source:");
        if let Some(url) = &self.current_url {
            let response = self.client.get(url).send().ok();
            if let Some(resp) = response {
                if let Ok(text) = resp.text() {
                    println!("{}", text);
                    return Ok(());
                }
            }
        }
        println!("Unable to fetch page source");
        Ok(())
    }

    fn download_page(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(url) = &self.current_url {
            let response = self.client.get(url).send()?;
            let content = response.bytes()?;
            std::fs::write(filename, content)?;
            println!("Page downloaded to: {}", filename);
        }
        Ok(())
    }

    fn search_in_page(&self, query: &str) -> io::Result<()> {
        execute!(io::stdout(), Clear(ClearType::All))?;

        execute!(
            io::stdout(),
            SetBackgroundColor(Color::Yellow),
            SetForegroundColor(Color::Black)
        )?;
        println!(" Search Results: \"{}\" ", query);
        execute!(io::stdout(), ResetColor)?;
        println!();

        let lines: Vec<&str> = self.page_content.lines().collect();
        let mut found = false;

        for (i, line) in lines.iter().enumerate() {
            if line.to_lowercase().contains(&query.to_lowercase()) {
                found = true;

                execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                print!("{:4} │ ", i + 1);

                let lower_line = line.to_lowercase();
                let lower_query = query.to_lowercase();
                let mut last_pos = 0;

                for (start, _) in lower_line.match_indices(&lower_query) {
                    execute!(io::stdout(), ResetColor)?;
                    print!("{}", &line[last_pos..start]);

                    execute!(
                        io::stdout(),
                        SetBackgroundColor(Color::Yellow),
                        SetForegroundColor(Color::Black)
                    )?;
                    print!("{}", &line[start..start + query.len()]);

                    last_pos = start + query.len();
                }

                execute!(io::stdout(), ResetColor)?;
                println!("{}", &line[last_pos..]);
            }
        }

        if !found {
            execute!(io::stdout(), SetForegroundColor(Color::Red))?;
            println!("No matches found.");
        }

        execute!(io::stdout(), ResetColor)?;
        println!("\nPress any key to return...");
        Ok(())
    }

    fn toggle_raw_mode(&self) -> io::Result<()> {
        execute!(io::stdout(), Clear(ClearType::All))?;
        println!("{}", self.page_content);
        println!("\nPress any key to return to normal mode...");
        io::stdout().flush()?;

        enable_raw_mode()?;
        let _ = read()?;
        disable_raw_mode()?;

        self.display_page()?;
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut browser = Browser::new();
    println!("Welcome to the Rust Web Browser!");
    println!("Type 'h' for help.");

    loop {
        print!("\nCommand: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "q" | "quit" => break,
            "h" | "help" => {
                println!("Commands:");
                println!("g URL      - Go to URL");
                println!("b         - Show bookmarks");
                println!("a TITLE   - Add current page to bookmarks");
                println!("h         - Show this help");
                println!("history   - Show history");
                println!("r         - Reload current page");
                println!("source    - View page source");
                println!("raw       - Toggle raw mode view");
                println!("download FILENAME - Download current page");
                println!("search QUERY - Search in current page");
                println!("w         - Scroll up");
                println!("s         - Scroll down");
                println!("q         - Quit");
            }
            "w" => {
                if browser.scroll_position >= 5 {
                    browser.scroll_position -= 5;
                } else {
                    browser.scroll_position = 0;
                }
                browser.display_page()?;
            }
            "s" => {
                let lines = browser.page_content.lines().count();
                let terminal_height = crossterm::terminal::size()?.1 as usize - 7;
                let max_scroll = if lines > terminal_height {
                    lines - terminal_height
                } else {
                    0
                };

                browser.scroll_position = std::cmp::min(browser.scroll_position + 5, max_scroll);
                browser.display_page()?;
            }
            "b" => browser.show_bookmarks()?,
            "history" => browser.show_history()?,
            "r" => {
                if let Some(url) = browser.current_url.clone() {
                    browser.navigate(&url)?;
                }
            }
            input if input.starts_with("g ") => {
                let url = input[2..].trim();
                if let Err(e) = browser.navigate(url) {
                    println!("Error: {}", e);
                }
            }
            input if input.starts_with("a ") => {
                let title = input[2..].trim();
                if let Err(e) = browser.add_bookmark(title) {
                    println!("Error adding bookmark: {}", e);
                }
            }
            "source" => browser.view_page_source()?,
            "raw" => browser.toggle_raw_mode()?,

            input if input.starts_with("download ") => {
                let filename = input[9..].trim();
                if let Err(e) = browser.download_page(filename) {
                    println!("Error downloading page: {}", e);
                }
            }

            input if input.starts_with("search ") => {
                let query = input[7..].trim();
                browser.search_in_page(query)?;
            }

            _ => println!("Unknown command. Press 'h' for help."),
        }
    }

    Ok(())
}
