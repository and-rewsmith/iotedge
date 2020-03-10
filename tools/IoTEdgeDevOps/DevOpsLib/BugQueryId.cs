// Copyright (c) Microsoft. All rights reserved.
using System.Collections.Generic;

namespace DevOpsLib
{
    public static class BugQueryId
    {
        public const string SampleQuery = "d7f4e036-0ccb-46a6-a33d-87a932296543";

        public readonly static Dictionary<string, string> QueryNamestoIds = new Dictionary<string, string>() 
        { 
            {nameof(SampleQuery), SampleQuery  } 
        };
    }
}
