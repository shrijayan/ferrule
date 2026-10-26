using System;
using System.Net.Http;
using System.Threading;
using System.Threading.Tasks;
using Acme.Core;
using Acme.Exceptions;
using Acme.Models.Widgets.Parts;

namespace Acme.Services.Widgets;

/// <inheritdoc/>
public sealed class PartService : IPartService
{
    readonly Lazy<IPartServiceWithRawResponse> _withRawResponse;

    /// <inheritdoc/>
    public IPartServiceWithRawResponse WithRawResponse
    {
        get { return _withRawResponse.Value; }
    }

    readonly IAcmeClient _client;

    /// <inheritdoc/>
    public IPartService WithOptions(Func<ClientOptions, ClientOptions> modifier)
    {
        return new PartService(this._client.WithOptions(modifier));
    }

    public PartService(IAcmeClient client)
    {
        _client = client;

        _withRawResponse = new(() => new PartServiceWithRawResponse(client.WithRawResponse));
    }

    /// <inheritdoc/>
    public async Task<PartRetrieveResponse> Retrieve(PartRetrieveParams parameters, CancellationToken cancellationToken = default)
    {
        using var response = await this
            .WithRawResponse.Retrieve(parameters, cancellationToken)
            .ConfigureAwait(false);
        return await response.Deserialize(cancellationToken).ConfigureAwait(false);
    }

    /// <inheritdoc/>
    public Task<PartRetrieveResponse> Retrieve(long partNumber, PartRetrieveParams parameters, CancellationToken cancellationToken = default)
    {
        return this.Retrieve(parameters with { PartNumber = partNumber }, cancellationToken);
    }
}

/// <inheritdoc/>
public sealed class PartServiceWithRawResponse : IPartServiceWithRawResponse
{
    readonly IAcmeClientWithRawResponse _client;

    /// <inheritdoc/>
    public IPartServiceWithRawResponse WithOptions(Func<ClientOptions, ClientOptions> modifier)
    {
        return new PartServiceWithRawResponse(this._client.WithOptions(modifier));
    }

    public PartServiceWithRawResponse(IAcmeClientWithRawResponse client)
    {
        _client = client;
    }

    /// <inheritdoc/>
    public async Task<HttpResponse<PartRetrieveResponse>> Retrieve(PartRetrieveParams parameters, CancellationToken cancellationToken = default)
    {
        if (parameters.PartNumber == null)
        {
            throw new AcmeInvalidDataException("'parameters.PartNumber' cannot be null");
        }

        HttpRequest<PartRetrieveParams> request = new()
        {
            Method = HttpMethod.Get,
            Params = parameters,
        };
        var response = await this._client.Execute(request, cancellationToken).ConfigureAwait(false);
        return new(
            response,
            async (token) =>
            {
                var part = await response
                    .Deserialize<PartRetrieveResponse>(token)
                    .ConfigureAwait(false);
                if (this._client.ResponseValidation)
                {
part.Validate();
                }
                return part;
            }
        );
    }

    /// <inheritdoc/>
    public Task<HttpResponse<PartRetrieveResponse>> Retrieve(long partNumber, PartRetrieveParams parameters, CancellationToken cancellationToken = default)
    {
        return this.Retrieve(parameters with { PartNumber = partNumber }, cancellationToken);
    }
}
