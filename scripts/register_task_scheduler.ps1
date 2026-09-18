# Register Aaroneous Passive Local Agent Engine in Windows Task Scheduler

$taskName = "Aaroneous-Devtools-Passive-Engine"
$scriptPath = "D:\aaroneous-devtools\scripts\local_agent_daemon.ps1"
$targetWorkspace = "D:\Aaroneous"
$queueFile = "D:\aaroneous-devtools\queue\task_queue.json"
$logFile = "D:\aaroneous-devtools\queue\agent_progress_log.md"

$arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$scriptPath`" -TargetWorkspace `"$targetWorkspace`" -QueueFile `"$queueFile`" -LogFile `"$logFile`" -SinglePass"

$action = New-ScheduledTaskAction -Execute "pwsh.exe" -Argument $arguments
$trigger = New-ScheduledTaskTrigger -Once -At (Get-Date) -RepetitionInterval (New-TimeSpan -Minutes 15)
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable

Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger -Settings $settings -Force
Write-Host "Successfully registered scheduled task '$taskName' in Windows Task Scheduler!"
