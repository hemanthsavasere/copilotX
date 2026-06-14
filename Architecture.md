<img src="https://r2cdn.perplexity.ai/pplx-full-logo-primary-dark%402x.png" style="height:64px;margin-right:32px"/>

# Architeecture of the repo [https://github.com/sohzm/cheating-daddy](https://github.com/sohzm/cheating-daddy)

Here is a comprehensive architectural breakdown of the [sohzm/cheating-daddy](https://github.com/sohzm/cheating-daddy) repository.

## Overview

**cheating-daddy** is a real-time AI interview/meeting assistant built as an **Electron desktop app** that captures screen + audio and feeds it to Google Gemini 2.0 Flash Live to provide contextual, real-time help during interviews, sales calls, and meetings.

## Tech Stack

| Layer | Technology |
| :-- | :-- |
| Shell | Electron (via Electron Forge) |
| UI | Vanilla JS + HTML (no framework) |
| AI Backend | Google Gemini 2.0 Flash Live API |
| Local AI | Ollama (local model fallback) |
| Packaging | `forge.config.js` for macOS/Windows builds |
| Persistence | Local storage (`storage.js`) |

## Project Structure

```
cheating-daddy/
├── forge.config.js          # Electron Forge build config (macOS/Windows packaging)
├── entitlements.plist        # macOS app sandbox permissions
├── AGENTS.md                 # Agent/AI instructions doc
├── src/
│   ├── index.js              # Main process (Electron entry point)
│   ├── index.html            # Renderer HTML shell
│   ├── preload.js            # Electron context bridge
│   ├── audioUtils.js         # Audio capture abstraction
│   ├── storage.js            # Local settings/session persistence
│   ├── assets/               # Static assets (icons, images)
│   ├── components/
│   │   ├── index.js          # Component registry/router
│   │   ├── app/              # App-level component(s)
│   │   └── views/            # Individual page views
│   │       ├── MainView.js         # Primary dashboard (34KB - largest view)
│   │       ├── AssistantView.js    # Live AI response overlay
│   │       ├── CustomizeView.js    # Profile & settings (27KB)
│   │       ├── AICustomizeView.js  # AI-specific tuning
│   │       ├── OnboardingView.js   # First-run setup
│   │       ├── HistoryView.js      # Session history
│   │       ├── HelpView.js         # Help content
│   │       ├── FeedbackView.js     # Feedback form
│   │       └── sharedPageStyles.js # Shared CSS-in-JS styles
│   └── utils/
│       ├── gemini.js         # Gemini 2.0 Flash Live integration (39KB)
│       ├── localai.js        # Local Ollama/local model integration
│       ├── renderer.js       # Rendering utilities (38KB)
│       ├── prompts.js        # Prompt templates per profile
│       ├── cloud.js          # Cloud sync/storage utilities
│       └── window.js         # Electron window management
```


## Core Architecture Patterns

**Main ↔ Renderer IPC (Electron model):** `src/index.js` is the Electron **main process** handling OS-level operations (screen capture, window control, audio), while `index.html` + the component views run in the **renderer process**.  The `preload.js` acts as the secure context bridge between them.

**View-based SPA routing:** The `src/components/views/` directory implements a single-page app pattern where the component router in `src/components/index.js` swaps views (MainView → AssistantView → HistoryView, etc.) without page reloads.

**Dual AI backend:** `src/utils/gemini.js` (39KB) handles the cloud path via Google Gemini 2.0 Flash Live for real-time multimodal streaming, while `src/utils/localai.js` (14KB) provides a fallback to locally running Ollama models.

**Profile-based prompting:** `src/utils/prompts.js` stores separate prompt templates for each use-case profile (Interview, Sales Call, Business Meeting, Presentation, Negotiation), which get injected into the Gemini/local AI calls.

**Audio capture layer:** `src/audioUtils.js` abstracts platform-specific audio — on macOS it uses [SystemAudioDump](https://github.com/Mohammed-Yasin-Mulla/Sound), on Windows it uses loopback capture, and on Linux it falls back to the microphone.

**Persistence:** `src/storage.js` (14KB) handles all local state — API keys, profile configs, session history — stored on-device without any mandatory cloud dependency.

