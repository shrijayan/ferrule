using System.Net.Http;

namespace __FerruleNs__.Exceptions;

public class __FerruleClient__UnprocessableEntityException : __FerruleClient__4xxException
{
    public __FerruleClient__UnprocessableEntityException(HttpRequestException? innerException = null)
        : base(innerException) { }
}
