Output "/tmp/demo.gif"
Set Shell "bash"
Set FontSize 24
Set Width 1200
Set Height 800
Set Margin 20
Set MarginFill "#674EFF"
Set BorderRadius 10
Set Theme "Catppuccin Frappe"
Set Framerate 60
Set PlaybackSpeed 0.5
Set CursorBlink false

Hide
Type 'export PATH="$PWD/target/release:$PATH"'
Enter
Type 'export DCDR_ADDR=http://192.168.64.1:21001'
Enter
Ctrl+L
Show

Type "decodering-cli osl capabilities get"
Enter
Sleep 2s

Type "decodering-cli osl secrets put"
Enter
Type "019ed44d-3eae-7c83-8d66-9aa5d7d937bf"
Enter
Type "prod-postgres"
Enter
Type "openbao-rs"
Enter
Type "acme/production/postgres"
Enter
Type "db-user"
Enter
Type "decodering"
Enter
Type "db-password"
Enter
Type "s3cr3t-demo-pw"
Enter
Type "db-host"
Enter
Type "postgres.internal:5432"
Enter
Enter
Enter
Sleep 2s

Type "decodering-cli osl secrets get"
Enter
Type "019ed44d-3eae-7c83-8d66-9aa5d7d937bf"
Enter
Type "prod-postgres"
Enter
Enter
Sleep 3s
