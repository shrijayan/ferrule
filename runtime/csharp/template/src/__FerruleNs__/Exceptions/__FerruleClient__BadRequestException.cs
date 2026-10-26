using System.Net.Http;

namespace __FerruleNs__.Exceptions;

public class __FerruleClient__BadRequestException : __FerruleClient__4xxException
{
    public __FerruleClient__BadRequestException(HttpRequestException? innerException = null)
        : base(innerException) { }
}
