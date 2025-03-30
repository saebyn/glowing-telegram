## Workflow Overview

```mermaid
flowchart TD
    A([Start])
    B{Enter series details}
    C[Pre-create stream series]
    D[Start stream tracker]
    E[Sync stream info to Twitch]
    F[Wait for stream to complete]
    G[Sync stream data including recording info]
    H[Extract video metadata, preview image]
    I[Save stream metadata, wait for later review]
    J{Wait / user returns}
    K[Present metadata to user]
    L{Proceed with analysis?}
    M[Highlighting of entries - optional]
    N[Transcription & analysis via OpenAI Whisper]
    O[Wait / user returns to metadata]
    P[Review each detection]
    Q[Render selection as rough cut]
    R[Export/upload]

    A --> B --> C --> D --> E --> F --> G --> H --> I --> J
    J --> K --> L
    L -- No --> I
    L -- Yes --> M --> N --> O --> P --> Q --> R