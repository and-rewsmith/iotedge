// Copyright (c) Microsoft. All rights reserved.
namespace TestResultCoordinator.Reports
{
    using Microsoft.Azure.Devices.Edge.ModuleUtil;
    using Microsoft.Azure.Devices.Edge.Util;

    class DeploymentTestReportMetadata : ITestReportMetadata
    {
        public DeploymentTestReportMetadata(string expectedSource, string actualSource, string testDescription)
        {
            this.ExpectedSource = Preconditions.CheckNonWhiteSpace(expectedSource, nameof(expectedSource));
            this.ActualSource = Preconditions.CheckNonWhiteSpace(actualSource, nameof(actualSource));
            this.TestDescription = Preconditions.CheckNonWhiteSpace(testDescription, nameof(testDescription));
        }

        public string ExpectedSource { get; }

        public string ActualSource { get; }

        public TestReportType TestReportType => TestReportType.DeploymentTestReport;

        public TestOperationResultType TestOperationResultType => TestOperationResultType.Deployment;

        public string[] ResultSources => new string[] { this.ExpectedSource, this.ActualSource };

        public string TestDescription { get; }

        public override string ToString()
        {
            return $"ExpectedSource: {this.ExpectedSource}, ActualSource: {this.ActualSource}, TestDescription: {this.TestDescription}, TestOperationResultType: {this.TestOperationResultType.ToString()}, ReportType: {this.TestReportType.ToString()}";
        }
    }
}
