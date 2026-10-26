using System;
using System.Net.Http;
using System.Threading;
using System.Threading.Tasks;
using Acme.Core;
using Acme.Exceptions;
using Acme.Models.Widgets;
using Acme.Services.Widgets;

namespace Acme.Services;

/// <inheritdoc/>
public sealed class WidgetService : IWidgetService
{
    readonly Lazy<IWidgetServiceWithRawResponse> _withRawResponse;

    /// <inheritdoc/>
    public IWidgetServiceWithRawResponse WithRawResponse
    {
        get { return _withRawResponse.Value; }
    }

    readonly IAcmeClient _client;

    /// <inheritdoc/>
    public IWidgetService WithOptions(Func<ClientOptions, ClientOptions> modifier)
    {
        return new WidgetService(this._client.WithOptions(modifier));
    }

    public WidgetService(IAcmeClient client)
    {
        _client = client;

        _withRawResponse = new(() => new WidgetServiceWithRawResponse(client.WithRawResponse));
        _parts = new(() => new PartService(client));
    }

    readonly Lazy<IPartService> _parts;
    public IPartService Parts
    {
        get { return _parts.Value; }
    }

    /// <inheritdoc/>
    public async Task<Widget> Create(WidgetCreateParams parameters, CancellationToken cancellationToken = default)
    {
        using var response = await this
            .WithRawResponse.Create(parameters, cancellationToken)
            .ConfigureAwait(false);
        return await response.Deserialize(cancellationToken).ConfigureAwait(false);
    }

    /// <inheritdoc/>
    public async Task<Widget> Retrieve(WidgetRetrieveParams parameters, CancellationToken cancellationToken = default)
    {
        using var response = await this
            .WithRawResponse.Retrieve(parameters, cancellationToken)
            .ConfigureAwait(false);
        return await response.Deserialize(cancellationToken).ConfigureAwait(false);
    }

    /// <inheritdoc/>
    public Task<Widget> Retrieve(string widgetID, WidgetRetrieveParams? parameters = null, CancellationToken cancellationToken = default)
    {
        parameters ??= new();

        return this.Retrieve(parameters with { WidgetID = widgetID }, cancellationToken);
    }

    /// <inheritdoc/>
    public async Task<WidgetListResponse> List(WidgetListParams? parameters = null, CancellationToken cancellationToken = default)
    {
        using var response = await this
            .WithRawResponse.List(parameters, cancellationToken)
            .ConfigureAwait(false);
        return await response.Deserialize(cancellationToken).ConfigureAwait(false);
    }

    /// <inheritdoc/>
    public Task Delete(WidgetDeleteParams parameters, CancellationToken cancellationToken = default)
    {
        return this.WithRawResponse.Delete(parameters, cancellationToken);
    }

    /// <inheritdoc/>
    public async Task Delete(string widgetID, WidgetDeleteParams? parameters = null, CancellationToken cancellationToken = default)
    {
        parameters ??= new();

        await this.Delete(parameters with { WidgetID = widgetID }, cancellationToken).ConfigureAwait(false);
    }
}

/// <inheritdoc/>
public sealed class WidgetServiceWithRawResponse : IWidgetServiceWithRawResponse
{
    readonly IAcmeClientWithRawResponse _client;

    /// <inheritdoc/>
    public IWidgetServiceWithRawResponse WithOptions(Func<ClientOptions, ClientOptions> modifier)
    {
        return new WidgetServiceWithRawResponse(this._client.WithOptions(modifier));
    }

    public WidgetServiceWithRawResponse(IAcmeClientWithRawResponse client)
    {
        _client = client;

        _parts = new(() => new PartServiceWithRawResponse(client));
    }

    readonly Lazy<IPartServiceWithRawResponse> _parts;
    public IPartServiceWithRawResponse Parts
    {
        get { return _parts.Value; }
    }

    /// <inheritdoc/>
    public async Task<HttpResponse<Widget>> Create(WidgetCreateParams parameters, CancellationToken cancellationToken = default)
    {
        HttpRequest<WidgetCreateParams> request = new()
        {
            Method = HttpMethod.Post,
            Params = parameters,
        };
        var response = await this._client.Execute(request, cancellationToken).ConfigureAwait(false);
        return new(
            response,
            async (token) =>
            {
                var widget = await response
                    .Deserialize<Widget>(token)
                    .ConfigureAwait(false);
                if (this._client.ResponseValidation)
                {
widget.Validate();
                }
                return widget;
            }
        );
    }

    /// <inheritdoc/>
    public async Task<HttpResponse<Widget>> Retrieve(WidgetRetrieveParams parameters, CancellationToken cancellationToken = default)
    {
        if (parameters.WidgetID == null)
        {
            throw new AcmeInvalidDataException("'parameters.WidgetID' cannot be null");
        }

        HttpRequest<WidgetRetrieveParams> request = new()
        {
            Method = HttpMethod.Get,
            Params = parameters,
        };
        var response = await this._client.Execute(request, cancellationToken).ConfigureAwait(false);
        return new(
            response,
            async (token) =>
            {
                var widget = await response
                    .Deserialize<Widget>(token)
                    .ConfigureAwait(false);
                if (this._client.ResponseValidation)
                {
widget.Validate();
                }
                return widget;
            }
        );
    }

    /// <inheritdoc/>
    public Task<HttpResponse<Widget>> Retrieve(string widgetID, WidgetRetrieveParams? parameters = null, CancellationToken cancellationToken = default)
    {
        parameters ??= new();

        return this.Retrieve(parameters with { WidgetID = widgetID }, cancellationToken);
    }

    /// <inheritdoc/>
    public async Task<HttpResponse<WidgetListResponse>> List(WidgetListParams? parameters = null, CancellationToken cancellationToken = default)
    {
        parameters ??= new();

        HttpRequest<WidgetListParams> request = new()
        {
            Method = HttpMethod.Get,
            Params = parameters,
        };
        var response = await this._client.Execute(request, cancellationToken).ConfigureAwait(false);
        return new(
            response,
            async (token) =>
            {
                var widget = await response
                    .Deserialize<WidgetListResponse>(token)
                    .ConfigureAwait(false);
                if (this._client.ResponseValidation)
                {
widget.Validate();
                }
                return widget;
            }
        );
    }

    /// <inheritdoc/>
    public Task<HttpResponse> Delete(WidgetDeleteParams parameters, CancellationToken cancellationToken = default)
    {
        if (parameters.WidgetID == null)
        {
            throw new AcmeInvalidDataException("'parameters.WidgetID' cannot be null");
        }

        HttpRequest<WidgetDeleteParams> request = new()
        {
            Method = HttpMethod.Delete,
            Params = parameters,
        };
        return this._client.Execute(request, cancellationToken);
    }

    /// <inheritdoc/>
    public Task<HttpResponse> Delete(string widgetID, WidgetDeleteParams? parameters = null, CancellationToken cancellationToken = default)
    {
        parameters ??= new();

        return this.Delete(parameters with { WidgetID = widgetID }, cancellationToken);
    }
}
