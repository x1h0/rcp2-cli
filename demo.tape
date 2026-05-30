# Install vhs: https://github.com/charmbracelet/vhs

Output demo.gif

Set Shell "bash"
Set FontSize 14
Set Width 1200
Set Height 700
Set Theme "Catppuccin Mocha"
Set Padding 20
Set TypingSpeed 0ms

# Start
Type "RCP2_ACCEPT_RISK=1 RCP2_ALLOW_SEND=1 rcp2-cli tui"
Enter
Sleep 3s

# Browse pads
Down
Sleep 800ms
Down
Sleep 800ms
Down
Sleep 800ms

# Switch to bank 2
Right
Sleep 1.5s

# Back to bank 1
Left
Sleep 1s

# Select first pad
Up
Sleep 500ms
Up
Sleep 500ms
Up
Sleep 1s

# Show help overlay
Type "?"
Sleep 3s

# Close help
Type "?"
Sleep 1s

# Switch to monitor view
Type "m"
Sleep 3s

# Back to pads
Escape
Sleep 1.5s

# Quit
Type "q"
