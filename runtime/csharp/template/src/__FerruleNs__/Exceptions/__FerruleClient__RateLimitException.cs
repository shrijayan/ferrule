using System.Net.Http;

namespace __FerruleNs__.Exceptions;

public class __FerruleClient__RateLimitException : __FerruleClient__4xxException
{
    public __FerruleClient__RateLimitException(HttpRequestException? innerException = null)
        : base(innerException) { }
}
