using System;
using System.Net.Http;

namespace __FerruleNs__.Exceptions;

public class __FerruleClient__Exception : Exception
{
    public __FerruleClient__Exception(string message, Exception? innerException = null)
        : base(message, innerException) { }

    protected __FerruleClient__Exception(HttpRequestException? innerException)
        : base(null, innerException) { }
}
