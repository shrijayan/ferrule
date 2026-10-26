using System.Net.Http;

namespace __FerruleNs__.Exceptions;

public class __FerruleClient__4xxException : __FerruleClient__ApiException
{
    public __FerruleClient__4xxException(HttpRequestException? innerException = null)
        : base(innerException) { }
}
