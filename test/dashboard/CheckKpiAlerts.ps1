Param (
    [ValidateNotNullOrEmpty()]
    [string] $User = $null,

    [ValidateNotNullOrEmpty()]
    [string] $Password = $null,

    [ValidateNotNullOrEmpty()]
    [string] $Tenant = $null
)

az login --service-principal -u $User -p $Password --tenant $Tenant --verbose

$alertNames = Get-Content .\alertNames.txt
$alertNames = $alertNames -Join "', '"
$query = "Alert | where AlertName in ('$alertNames')"

$query
$triggeredAlerts = az monitor log-analytics query -w fdf47b96-87f3-4b86-90b9-d83e2deae8a0 --analytics-query $query --verbose

$triggeredAlerts > ./temp.json