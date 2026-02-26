# exec_and_record scenarios

## User Stories
- As a tool integrator, I want to record a command and get an MP4 by default so I can attach the video to an LLM review.
- As a debugger, I want a plain text transcript in addition to video so I can paste logs into a model without heavy context.
- As a power user, I want to control output naming and location so I can organize artifacts per run.
- As a CI author, I want the command to fail fast if required tools are missing so pipelines are reliable.

## Scenarios
1. Default MP4 recording
   - Given a user runs `exec_and_record -- codex`
   - When the command completes
   - Then `./recordings/session_YYYYMMDD_HHMMSS.mp4` exists

2. Multiple output formats
   - Given a user runs `exec_and_record --format mp4,txt,raw -- claude`
   - When the command completes
   - Then `.mp4`, `.txt`, and `.raw.log` are present with the same base name

3. Custom output path
   - Given a user runs `exec_and_record --output ./recordings/demo_run.mp4 -- gemini`
   - When the command completes
   - Then outputs are written to `./recordings/demo_run.*`

4. Custom directory and base name
   - Given a user runs `exec_and_record --out-dir ./artifacts --name run_42 -- codex`
   - When the command completes
   - Then outputs are written to `./artifacts/run_42.*`

5. Missing tool failure
   - Given `ffmpeg` is not installed and the user requests `--format mp4`
   - When the command starts
   - Then the process exits with a non-zero status and an error stating `ffmpeg` is missing
