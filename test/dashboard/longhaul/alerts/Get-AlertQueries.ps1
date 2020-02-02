Import-Module Az.Monitor

$MegaByteConstant = 1000000;
$AlertingInterval = 15 # TODO: take as input
$NumberOfMetrics = 40 # TODO: define constants better

# TODO: clean this up by making a func that returns this object
$GreaterThanZero = New-AzScheduledQueryRuleTriggerCondition `
   -ThresholdOperator "GreaterThan" `
   -Threshold "0"
$LessThanTwo = New-AzScheduledQueryRuleTriggerCondition `
   -ThresholdOperator "LessThan" `
   -Threshold "2" 
$LessThanThree = New-AzScheduledQueryRuleTriggerCondition `
   -ThresholdOperator "LessThan" `
   -Threshold "3" 
$LessThanFourty = New-AzScheduledQueryRuleTriggerCondition `
   -ThresholdOperator "LessThan" `
   -Threshold "40" 
$GreaterThanFourty = New-AzScheduledQueryRuleTriggerCondition `
   -ThresholdOperator "GreaterThan" `
   -Threshold "40" 

class Alert{
   [ValidateNotNullOrEmpty()][string]$Name
   [ValidateNotNullOrEmpty()][string]$Query
   [ValidateNotNullOrEmpty()]$Comparator #TODO: find way to import type (module manifest?)
   [ValidateNotNullOrEmpty()]$Threshold
}
$Alerts = New-Object System.Collections.Generic.List[Alert]

$LoadGenMessagesPerMinThreshold = 45
$TempFilterMessagesPerMinThreshold = 9 
$UpstreamMessageRateAlertQuery = Get-Content -Path ".\queries\UpstreamMessageRate.kql" 
$UpstreamMessageRateAlertQuery = $UpstreamMessageRateAlertQuery.Replace("<LOADGEN.THRESHOLD>", $LoadGenMessagesPerMinThreshold)
$UpstreamMessageRateAlertQuery = $UpstreamMessageRateAlertQuery.Replace("<TEMPFILTER.THRESHOLD>", $TempFilterMessagesPerMinThreshold)
$UpstreamMessageRateAlertQuery = $UpstreamMessageRateAlertQuery.Replace("<ALERTING.INTERVAL>", $AlertingInterval)
$UpstreamMessageRate = [Alert]@{
   Name = "upstream-message-rate"
   Query = $UpstreamMessageRateAlertQuery
   Comparator = $GreaterThanZero
   Threshold = "threshold0: $LoadGenMessagesPerMinThreshold threshold1: $TempFilterMessagesPerMinThreshold"
}
$Alerts.Add($UpstreamMessageRate)

$TempSensorMessagesPerMinThreshold = 9 
$LocalMessageRateAlertQuery = Get-Content -Path ".\queries\LocalMessageRate.kql" 
$LocalMessageRateAlertQuery = $LocalMessageRateAlertQuery.Replace("<TEMPSENSOR.THRESHOLD>", $TempSensorMessagesPerMinThreshold)
$LocalMessageRateAlertQuery = $LocalMessageRateAlertQuery.Replace("<ALERTING.INTERVAL>", $AlertingInterval)
$LocalMessageRate = [Alert]@{
   Name = "local-message-rate"
   Query = $LocalMessageRateAlertQuery
   Comparator = $GreaterThanZero
   Threshold = "threshold0: $TempSensorMessagesPerMinThreshold"
}
$Alerts.Add($LocalMessageRate)

$NoUpstreamMessagesQuery = Get-Content -Path ".\queries\NoUpstreamMessages.kql" 
$NoUpstreamMessagesQuery = $NoUpstreamMessagesQuery.Replace("<ALERTING.INTERVAL>", $AlertingInterval)
$NoUpstreamMessages  = [Alert]@{
   Name = "no-upstream-messages"
   Query = $NoUpstreamMessagesQuery
   Comparator = $LessThanTwo
   Threshold = "threshold0: 0"
}
$Alerts.Add($NoUpstreamMessages)

$NoLocalMessagesQuery = Get-Content -Path ".\queries\NoLocalMessages.kql" 
$NoLocalMessages  = [Alert]@{
   Name = "no-local-messages"
   Query = $NoLocalMessagesQuery
   Comparator = $LessThanTwo
   Threshold = "threshold0: 0"
}
$Alerts.Add($NoLocalMessages)

$TwinTesterUpstreamReportedPropertyUpdatesPerMinThreshold = 5 
$ReportedPropertyRateAlertQuery = Get-Content -Path ".\queries\ReportedPropertyRate.kql" 
$ReportedPropertyRateAlertQuery = $ReportedPropertyRateAlertQuery.Replace("<ALERTING.INTERVAL>", $AlertingInterval)
$ReportedPropertyRateAlertQuery = $ReportedPropertyRateAlertQuery.Replace("<TWINTESTER.THRESHOLD>", $TwinTesterUpstreamReportedPropertyUpdatesPerMinThreshold)
$ReportedPropertyRate = [Alert]@{
   Name = "reported-property-rate"
   Query = $ReportedPropertyRateAlertQuery
   Comparator = $GreaterThanZero
   Threshold = "threshold0: $TwinTesterUpstreamReportedPropertyUpdatesPerMinThreshold"
}
$Alerts.Add($ReportedPropertyRate)

$NoReportedPropertiesQuery = Get-Content -Path ".\queries\NoReportedProperties.kql" 
$NoReportedPropertiesQuery = $NoReportedPropertiesQuery.Replace("<ALERTING.INTERVAL>", $AlertingInterval)
$NoReportedProperties  = [Alert]@{
   Name = "no-reported-properties"
   Query = $NoReportedPropertiesQuery
   Comparator = $LessThanThree
   Threshold = "threshold0: 0"
}
$Alerts.Add($NoReportedProperties)

$QueueLengthThreshold = 100 
$QueueLengthAlertQuery = Get-Content -Path ".\queries\QueueLength.kql" 
$QueueLengthAlertQuery = $QueueLengthAlertQuery.Replace("<QUEUELENGTH.THRESHOLD>", $QueueLengthThreshold)
$QueueLengthAlertQuery = $QueueLengthAlertQuery.Replace("<ALERTING.INTERVAL>", $AlertingInterval)
$QueueLength = [Alert]@{
   Name = "queue-length"
   Query = $QueueLengthAlertQuery
   Comparator = $GreaterThanZero
   Threshold = "threshold0: $QueueLengthThreshold"
}
$Alerts.Add($QueueLength)

$EdgeAgentCpuThreshold = .90 
$EdgeAgentCpuAlertQuery = Get-Content -Path ".\queries\EdgeAgentCpu.kql" 
$EdgeAgentCpuAlertQuery = $EdgeAgentCpuAlertQuery.Replace("<CPU.THRESHOLD>", $EdgeAgentCpuThreshold)
$EdgeAgentCpuAlertQuery = $EdgeAgentCpuAlertQuery.Replace("<ALERTING.INTERVAL>", $AlertingInterval)
$EdgeAgentCpu = [Alert]@{
   Name = "edge-agent-cpu"
   Query = $EdgeAgentCpuAlertQuery
   Comparator = $GreaterThanZero
   Threshold = "threshold0: $EdgeAgentCpuThreshold"
}
$Alerts.Add($EdgeAgentCpu)

$EdgeHubCpuThreshold = .90 
$EdgeHubCpuAlertQuery = Get-Content -Path ".\queries\EdgeHubCpu.kql" 
$EdgeHubCpuAlertQuery = $EdgeHubCpuAlertQuery.Replace("<CPU.THRESHOLD>", $EdgeHubCpuThreshold)
$EdgeHubCpuAlertQuery = $EdgeHubCpuAlertQuery.Replace("<ALERTING.INTERVAL>", $AlertingInterval)
$EdgeHubCpu = [Alert]@{
   Name = "edge-hub-cpu"
   Query = $EdgeHubCpuAlertQuery
   Comparator = $GreaterThanZero
   Threshold = "threshold0: $EdgeHubCpuThreshold"
}
$Alerts.Add($EdgeHubCpu)

$EdgeAgentMemoryThresholdArm = 100 * $MegaByteConstant
$EdgeAgentMemoryThresholdAmd = 300 * $MegaByteConstant
$EdgeAgentMemoryQuery = Get-Content -Path ".\queries\EdgeAgentMemory.kql" 
$EdgeAgentMemoryQuery = $EdgeAgentMemoryQuery.Replace("<MEMORY.THRESHOLD.ARM>", $EdgeAgentMemoryThresholdArm)
$EdgeAgentMemoryQuery = $EdgeAgentMemoryQuery.Replace("<MEMORY.THRESHOLD.AMD>", $EdgeAgentMemoryThresholdAmd)
$EdgeAgentMemoryQuery = $EdgeAgentMemoryQuery.Replace("<ALERTING.INTERVAL>", $AlertingInterval)
$EdgeAgentMemory = [Alert]@{
   Name = "edge-agent-memory"
   Query = $EdgeAgentMemoryQuery
   Comparator = $GreaterThanZero
   Threshold = "threshold0: $EdgeAgentMemoryThreshold"
}
$Alerts.Add($EdgeAgentMemory)

$EdgeHubMemoryThresholdArm = 250 * $MegaByteConstant 
$EdgeHubMemoryThresholdAmd = 900 * $MegaByteConstant 
$EdgeHubMemoryQuery = Get-Content -Path ".\queries\EdgeHubMemory.kql" 
$EdgeHubMemoryQuery = $EdgeHubMemoryQuery.Replace("<MEMORY.THRESHOLD.ARM>", $EdgeHubMemoryThresholdArm)
$EdgeHubMemoryQuery = $EdgeHubMemoryQuery.Replace("<MEMORY.THRESHOLD.AMD>", $EdgeHubMemoryThresholdAmd)
$EdgeHubMemoryQuery = $EdgeHubMemoryQuery.Replace("<ALERTING.INTERVAL>", $AlertingInterval)
$EdgeHubMemory = [Alert]@{
   Name = "edge-hub-memory"
   Query = $EdgeHubMemoryQuery
   Comparator = $GreaterThanZero
   Threshold = "threshold0: $EdgeHubMemoryThreshold"
}
$Alerts.Add($EdgeHubMemory)

$NumberOfMetricsQuery = Get-Content -Path ".\queries\NumberOfMetrics.kql" 
$NumberOfMetricsQuery = $NumberOfMetricsQuery.Replace("<ALERTING.INTERVAL>", $AlertingInterval)
$NumberOfMetricsTooLow = [Alert]@{
   Name = "number-of-metrics-too-low"
   Query = $NumberOfMetricsQuery
   Comparator = $LessThanFourty
   Threshold = "threshold0: 42"
}
$NumberOfMetricsTooHigh = [Alert]@{
   Name = "number-of-metrics-too-high"
   Query = $NumberOfMetricsQuery
   Comparator = $GreaterThanFourty
   Threshold = "threshold0: 42"
}
$Alerts.Add($NumberOfMetricsTooLow)
$Alerts.Add($NumberOfMetricsTooHigh)

$ModuleStartThreshold = 100 
$ModuleStartsQuery = Get-Content -Path ".\queries\ModuleStarts.kql" 
$ModuleStartsQuery = $ModuleStartsQuery.Replace("<MODULESTARTS.THRESHOLD>", $ModuleStartThreshold)
$ModuleStartsQuery = $ModuleStartsQuery.Replace("<ALERTING.INTERVAL>", $AlertingInterval)
$ModuleStarts = [Alert]@{
   Name = "failed-module-starts"
   Query = $ModuleStartsQuery
   Comparator = $GreaterThanZero
   Threshold = "threshold0: $ModuleStartThreshold"
}
$Alerts.Add($ModuleStarts)

Write-Output $Alerts
