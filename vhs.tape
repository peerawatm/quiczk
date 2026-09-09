Output vhs.gif
Require quiczk
Set Shell zsh
Set FontSize 20
Set Width 1200
Set Height 750
Set TypingSpeed 75ms
Set Framerate 60
Set Padding 10

Hide
Type "export PATH=$HOME/.local/bin:$PATH"
Enter
Sleep 500ms
Type "mkdir -p .vhs-config/quiczk"
Enter
Sleep 500ms
Type "echo '[config]' > .vhs-config/quiczk/config.toml"
Enter
Sleep 500ms
Type "echo 'shuffle_questions = false' >> .vhs-config/quiczk/config.toml"
Enter
Sleep 500ms
Type "echo 'shuffle_options = false' >> .vhs-config/quiczk/config.toml"
Enter
Sleep 500ms
Type "echo 'random_start_cursor = false' >> .vhs-config/quiczk/config.toml"
Enter
Sleep 500ms
Type "echo 'hide_options_until_interact = true' >> .vhs-config/quiczk/config.toml"
Enter
Sleep 500ms
Type "export XDG_CONFIG_HOME=$PWD/.vhs-config && clear"
Enter
Sleep 1s
Show

Type@150ms "quiczk example.toml"
Sleep 1200ms
Enter
Wait+Screen /reveal options/
Sleep 1500ms
Enter
Wait+Screen /Peerawat Monviset/
Sleep 2s
Down
Sleep 800ms
Enter
Wait+Screen /next question/
Sleep 2s
Enter
Wait+Screen /reveal options/
Sleep 1200ms
Enter
Wait+Screen /How do you exit Vim/
Sleep 2s
Enter
Wait+Screen /Score:/
Sleep 3500ms
Type "q"
Sleep 1s
Hide
Type "rm -rf .vhs-config && clear"
Enter
Sleep 500ms
Show
