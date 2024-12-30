# Terminal Web Browser

A lightweight, terminal-based web browser built in Rust that combines efficiency with functionality.

## Usage Guide

- g [url] - Navigate to URL

    -  w - Scroll up (5 lines)

    -  s - Scroll down (5 lines)

    -  q - Exit browser

- a [title] - Add bookmark

- b - Access bookmarks

- history - View browsing history

- search - In-page search

- source - View page source

- raw - Toggle raw HTML view

- download - Save page locally

- r - Reload current page

## Key Features

### Navigation
- Fast page loading with minimal resource usage
- Vim-style navigation (`w`/`s` for 5-line scrolling)
- Direct URL access with `g` command

### Bookmarking System
- Quick bookmark addition with `a [title]`
- Numerical navigation (access bookmarks by number)
- Organized bookmark management
- Persistent storage across sessions

### History Management
- Automatic history tracking
- Quick access to previous pages
- Numerical navigation through history
- Session persistence

### Developer Tools
- View page source with `source` command
- Raw HTML viewing mode
- Ability to download page
- In-page text search with highlighting
