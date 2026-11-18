#Requires -RunAsAdministrator
<#
.SYNOPSIS
A locked SSH door into this Windows PC, so Oscyl can be tested on a real
synth from a Linux machine. Closed unless switched on.

.DESCRIPTION
  -Install -LinuxIp 192.168.1.18 -PublicKey 'ssh-ed25519 AAAA...'
      Once. OpenSSH Server set to manual start; a standard (not admin) user
      'oscyl-test' whose only way in is that key, from that address; a
      firewall rule for that address only. Everything stays closed.
  -On [-Hours 3]
      Start sshd and open the rule. It closes itself after -Hours.
  -Off  (also: no argument)
      Stop sshd, end every session of 'oscyl-test', close the rule.
  -Uninstall
      Off, then remove the user, its profile, the key, the rule and OpenSSH
      Server, and put back the sshd_config from before -Install.

Run from an admin PowerShell:
  powershell -ExecutionPolicy Bypass -File .\windows-test-host.ps1 -On
#>
[CmdletBinding(DefaultParameterSetName = 'Off')]
param(
    [Parameter(ParameterSetName = 'Install', Mandatory)] [switch] $Install,
    [Parameter(ParameterSetName = 'Install', Mandatory)] [string] $LinuxIp,
    [Parameter(ParameterSetName = 'Install', Mandatory)] [string] $PublicKey,
    [Parameter(ParameterSetName = 'On', Mandatory)] [switch] $On,
    [Parameter(ParameterSetName = 'On')] [double] $Hours = 3,
    [Parameter(ParameterSetName = 'Off')] [switch] $Off,
    [Parameter(ParameterSetName = 'Uninstall', Mandatory)] [switch] $Uninstall
)
$ErrorActionPreference = 'Stop'

$User = 'oscyl-test'
$Rule = 'Oscyl test SSH'
$Task = 'Oscyl test SSH off'
$Capability = 'OpenSSH.Server~~~~0.0.1.0'
$Config = "$env:ProgramData\ssh\sshd_config"
$Backup = "$Config.before-oscyl"
$Keys = "$env:ProgramData\ssh\oscyl_authorized_keys"
$Hidden = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon\SpecialAccounts\UserList'

# Off is also what the timer runs, so it is a self-contained command.
$CloseDoor = @"
Disable-NetFirewallRule -DisplayName '$Rule' -ErrorAction SilentlyContinue
Stop-Service sshd -Force -ErrorAction SilentlyContinue
Get-Process -IncludeUserName | Where-Object UserName -like '*\$User' | Stop-Process -Force -ErrorAction SilentlyContinue
Get-Process sshd, sshd-session -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Unregister-ScheduledTask -TaskName '$Task' -Confirm:`$false -ErrorAction SilentlyContinue
"@

function Close-Door { & ([scriptblock]::Create($CloseDoor)) }

switch ($PSCmdlet.ParameterSetName) {
    'Install' {
        if ($PublicKey -notmatch '^ssh-ed25519 [A-Za-z0-9+/=]+( .*)?$') {
            throw 'PublicKey must be one ssh-ed25519 public key line.'
        }
        if ((Get-WindowsCapability -Online -Name $Capability).State -ne 'Installed') {
            Add-WindowsCapability -Online -Name $Capability | Out-Null
        }
        # The capability's own rule lets anyone reach port 22: keep it off.
        Disable-NetFirewallRule -Name 'OpenSSH-Server-In-TCP' -ErrorAction SilentlyContinue
        Set-Service sshd -StartupType Manual
        # The first start makes the host keys and the default config.
        Start-Service sshd
        Stop-Service sshd -Force

        if (-not (Get-LocalUser $User -ErrorAction SilentlyContinue)) {
            # A password nobody knows: the key is the only way in.
            $bytes = [byte[]]::new(32)
            [Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($bytes)
            $password = ConvertTo-SecureString ([Convert]::ToBase64String($bytes) + 'Aa1!') -AsPlainText -Force
            New-LocalUser $User -Password $password -PasswordNeverExpires -UserMayNotChangePassword `
                -Description 'Oscyl tests over SSH from Linux' | Out-Null
            Add-LocalGroupMember -SID 'S-1-5-32-545' -Member $User  # Users
        }
        if (-not (Test-Path $Hidden)) { New-Item $Hidden -Force | Out-Null }
        Set-ItemProperty $Hidden -Name $User -Value 0 -Type DWord  # not on the sign-in screen

        # Only from the Linux machine; no forwarding of any kind.
        Set-Content $Keys "from=`"$LinuxIp`",restrict,pty $PublicKey" -Encoding ascii
        # sshd refuses a key file anyone but SYSTEM and Administrators can write.
        icacls $Keys /inheritance:r /grant '*S-1-5-18:F' /grant '*S-1-5-32-544:F' | Out-Null

        if (-not (Test-Path $Backup)) { Copy-Item $Config $Backup }
        Set-Content $Config -Encoding ascii @"
# Written by Oscyl's scripts/windows-test-host.ps1; the previous file is
# sshd_config.before-oscyl and comes back with -Uninstall.
AllowUsers $User
AuthenticationMethods publickey
PubkeyAuthentication yes
PasswordAuthentication no
KbdInteractiveAuthentication no
AuthorizedKeysFile __PROGRAMDATA__/ssh/oscyl_authorized_keys
AllowTcpForwarding no
AllowAgentForwarding no
AllowStreamLocalForwarding no
GatewayPorts no
PermitTunnel no
X11Forwarding no
Subsystem sftp sftp-server.exe
"@
        & "$env:WINDIR\System32\OpenSSH\sshd.exe" -t
        if ($LASTEXITCODE) { throw 'sshd rejected the new sshd_config.' }

        Remove-NetFirewallRule -DisplayName $Rule -ErrorAction SilentlyContinue
        New-NetFirewallRule -DisplayName $Rule -Direction Inbound -Protocol TCP -LocalPort 22 `
            -RemoteAddress $LinuxIp -Profile Private -Action Allow -Enabled False | Out-Null
        Close-Door
        "Installed and closed. Open with -On."
    }
    'On' {
        $until = (Get-Date).AddHours($Hours)
        $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($CloseDoor))
        $action = New-ScheduledTaskAction -Execute 'powershell.exe' `
            -Argument "-NoProfile -NonInteractive -EncodedCommand $encoded"
        Register-ScheduledTask -TaskName $Task -Action $action -User 'SYSTEM' -RunLevel Highest -Force `
            -Trigger (New-ScheduledTaskTrigger -Once -At $until) | Out-Null
        Enable-NetFirewallRule -DisplayName $Rule
        Start-Service sshd
        "Open until $($until.ToString('HH:mm')). Close sooner with -Off."
    }
    'Off' {
        Close-Door
        'Closed.'
    }
    'Uninstall' {
        Close-Door
        Remove-NetFirewallRule -DisplayName $Rule -ErrorAction SilentlyContinue
        Remove-Item $Keys -ErrorAction SilentlyContinue
        if (Test-Path $Backup) { Move-Item $Backup $Config -Force }
        Get-CimInstance Win32_UserProfile | Where-Object LocalPath -like "*\$User" | Remove-CimInstance
        Remove-LocalUser $User -ErrorAction SilentlyContinue
        Remove-ItemProperty $Hidden -Name $User -ErrorAction SilentlyContinue
        Remove-WindowsCapability -Online -Name $Capability | Out-Null
        'Removed.'
    }
}
