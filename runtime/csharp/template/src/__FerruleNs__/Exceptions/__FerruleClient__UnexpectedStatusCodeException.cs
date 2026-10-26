using System.Net.Http;

namespace __FerruleNs__.Exceptions;

public class __FerruleClient__UnexpectedStatusCodeException : __FerruleClient__ApiException
{
    public __FerruleClient__UnexpectedStatusCodeException(HttpRequestException? innerException = null)
        : base(innerException) { }
}
