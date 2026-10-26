using System;
using System.Threading;
using System.Threading.Tasks;
using Acme.Core;
using Acme.Models.Widgets.Parts;

namespace Acme.Services.Widgets;

/// <summary>
/// NOTE: Do not inherit from this type outside the SDK unless you're okay with breaking
/// changes in non-major versions. We may add new methods in the future that cause
/// existing derived classes to break.
/// </summary>
public interface IPartService
{
    /// <summary>
    /// Returns a view of this service that provides access to raw HTTP responses
    /// for each method.
    /// </summary>
    IPartServiceWithRawResponse WithRawResponse { get; }

    /// <summary>
    /// Returns a view of this service with the given option modifications applied.
    ///
    /// <para>The original service is not modified.</para>
    /// </summary>
    IPartService WithOptions(Func<ClientOptions, ClientOptions> modifier);

    /// <summary>
    /// Retrieve one part of a widget
    /// </summary>
    Task<PartRetrieveResponse> Retrieve(PartRetrieveParams parameters, CancellationToken cancellationToken = default);

    /// <inheritdoc cref="Retrieve(PartRetrieveParams, CancellationToken)"/>
    Task<PartRetrieveResponse> Retrieve(long partNumber, PartRetrieveParams parameters, CancellationToken cancellationToken = default);
}

/// <summary>
/// A view of <see cref="IPartService"/> that provides access to raw
/// HTTP responses for each method.
/// </summary>
public interface IPartServiceWithRawResponse
{
    /// <summary>
    /// Returns a view of this service with the given option modifications applied.
    ///
    /// <para>The original service is not modified.</para>
    /// </summary>
    IPartServiceWithRawResponse WithOptions(Func<ClientOptions, ClientOptions> modifier);

    /// <summary>
    /// Returns a raw HTTP response for <c>get /v1/widgets/{widget_id}/parts/{part_number}</c>, but is otherwise the
    /// same as <see cref="IPartService.Retrieve(PartRetrieveParams, CancellationToken)"/>.
    /// </summary>
    Task<HttpResponse<PartRetrieveResponse>> Retrieve(PartRetrieveParams parameters, CancellationToken cancellationToken = default);

    /// <inheritdoc cref="Retrieve(PartRetrieveParams, CancellationToken)"/>
    Task<HttpResponse<PartRetrieveResponse>> Retrieve(long partNumber, PartRetrieveParams parameters, CancellationToken cancellationToken = default);
}
