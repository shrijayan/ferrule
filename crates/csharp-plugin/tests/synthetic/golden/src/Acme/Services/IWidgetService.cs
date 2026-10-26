using System;
using System.Threading;
using System.Threading.Tasks;
using Acme.Core;
using Acme.Models.Widgets;
using Acme.Services.Widgets;

namespace Acme.Services;

/// <summary>
/// NOTE: Do not inherit from this type outside the SDK unless you're okay with breaking
/// changes in non-major versions. We may add new methods in the future that cause
/// existing derived classes to break.
/// </summary>
public interface IWidgetService
{
    /// <summary>
    /// Returns a view of this service that provides access to raw HTTP responses
    /// for each method.
    /// </summary>
    IWidgetServiceWithRawResponse WithRawResponse { get; }

    /// <summary>
    /// Returns a view of this service with the given option modifications applied.
    ///
    /// <para>The original service is not modified.</para>
    /// </summary>
    IWidgetService WithOptions(Func<ClientOptions, ClientOptions> modifier);

    IPartService Parts { get; }

    /// <summary>
    /// Creates a widget with the given name and parts. The SKU is assigned by the
    /// server.
    /// </summary>
    Task<Widget> Create(WidgetCreateParams parameters, CancellationToken cancellationToken = default);

    /// <summary>
    /// Retrieve a widget
    /// </summary>
    Task<Widget> Retrieve(WidgetRetrieveParams parameters, CancellationToken cancellationToken = default);

    /// <inheritdoc cref="Retrieve(WidgetRetrieveParams, CancellationToken)"/>
    Task<Widget> Retrieve(string widgetID, WidgetRetrieveParams? parameters = null, CancellationToken cancellationToken = default);

    /// <summary>
    /// List widgets
    /// </summary>
    Task<WidgetListResponse> List(WidgetListParams? parameters = null, CancellationToken cancellationToken = default);

    /// <summary>
    /// Delete a widget
    /// </summary>
    Task Delete(WidgetDeleteParams parameters, CancellationToken cancellationToken = default);

    /// <inheritdoc cref="Delete(WidgetDeleteParams, CancellationToken)"/>
    Task Delete(string widgetID, WidgetDeleteParams? parameters = null, CancellationToken cancellationToken = default);
}

/// <summary>
/// A view of <see cref="IWidgetService"/> that provides access to raw
/// HTTP responses for each method.
/// </summary>
public interface IWidgetServiceWithRawResponse
{
    /// <summary>
    /// Returns a view of this service with the given option modifications applied.
    ///
    /// <para>The original service is not modified.</para>
    /// </summary>
    IWidgetServiceWithRawResponse WithOptions(Func<ClientOptions, ClientOptions> modifier);

    IPartServiceWithRawResponse Parts { get; }

    /// <summary>
    /// Returns a raw HTTP response for <c>post /v1/widgets</c>, but is otherwise the
    /// same as <see cref="IWidgetService.Create(WidgetCreateParams, CancellationToken)"/>.
    /// </summary>
    Task<HttpResponse<Widget>> Create(WidgetCreateParams parameters, CancellationToken cancellationToken = default);

    /// <summary>
    /// Returns a raw HTTP response for <c>get /v1/widgets/{widget_id}</c>, but is otherwise the
    /// same as <see cref="IWidgetService.Retrieve(WidgetRetrieveParams, CancellationToken)"/>.
    /// </summary>
    Task<HttpResponse<Widget>> Retrieve(WidgetRetrieveParams parameters, CancellationToken cancellationToken = default);

    /// <inheritdoc cref="Retrieve(WidgetRetrieveParams, CancellationToken)"/>
    Task<HttpResponse<Widget>> Retrieve(string widgetID, WidgetRetrieveParams? parameters = null, CancellationToken cancellationToken = default);

    /// <summary>
    /// Returns a raw HTTP response for <c>get /v1/widgets</c>, but is otherwise the
    /// same as <see cref="IWidgetService.List(WidgetListParams?, CancellationToken)"/>.
    /// </summary>
    Task<HttpResponse<WidgetListResponse>> List(WidgetListParams? parameters = null, CancellationToken cancellationToken = default);

    /// <summary>
    /// Returns a raw HTTP response for <c>delete /v1/widgets/{widget_id}</c>, but is otherwise the
    /// same as <see cref="IWidgetService.Delete(WidgetDeleteParams, CancellationToken)"/>.
    /// </summary>
    Task<HttpResponse> Delete(WidgetDeleteParams parameters, CancellationToken cancellationToken = default);

    /// <inheritdoc cref="Delete(WidgetDeleteParams, CancellationToken)"/>
    Task<HttpResponse> Delete(string widgetID, WidgetDeleteParams? parameters = null, CancellationToken cancellationToken = default);
}
