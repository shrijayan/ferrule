using System.Collections.Frozen;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Diagnostics.CodeAnalysis;
using System.Net.Http;
using System.Text.Json;
using System.Text.Json.Serialization;
using Acme.Core;
using Acme.Exceptions;
using System = System;
using Text = System.Text;

namespace Acme.Models.Widgets;

/// <summary>
/// Creates a widget with the given name and parts. The SKU is assigned by the
/// server.
///
/// <para>NOTE: Do not inherit from this type outside the SDK unless you're okay with
/// breaking changes in non-major versions. We may add new methods in the future that
/// cause existing derived classes to break.</para>
/// </summary>
public record class WidgetCreateParams : ParamsBase
{
    readonly JsonDictionary _rawBodyData = new();
    public IReadOnlyDictionary<string, JsonElement> RawBodyData
    {
        get { return this._rawBodyData.Freeze(); }
    }

    /// <summary>
    /// The widget's display name.
    /// </summary>
    public required string Name
    {
        get
        {
            this._rawBodyData.Freeze();
            return this._rawBodyData.GetNotNullClass<string>("name");
        }
init { this._rawBodyData.Set("name", value); }
    }

    public ApiEnum<string, Color>? Color
    {
        get
        {
            this._rawBodyData.Freeze();
            return this._rawBodyData.GetNullableClass<ApiEnum<string, Color>>("color");
        }
init
{
if (value == null)
{
return;
}

this._rawBodyData.Set("color", value);
}
    }

    public Dimensions? Dimensions
    {
        get
        {
            this._rawBodyData.Freeze();
            return this._rawBodyData.GetNullableClass<Dimensions>("dimensions");
        }
init
{
if (value == null)
{
return;
}

this._rawBodyData.Set("dimensions", value);
}
    }

    public IReadOnlyList<string>? Tags
    {
        get
        {
            this._rawBodyData.Freeze();
            return this._rawBodyData.GetNullableStruct<ImmutableArray<string>>("tags");
        }
init
{
if (value == null)
{
return;
}

this._rawBodyData.Set<ImmutableArray<string>?>("tags", value == null ? null : ImmutableArray.ToImmutableArray(value));
}
    }

    public WidgetCreateParams() { }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    public WidgetCreateParams(WidgetCreateParams widgetCreateParams)
        : base(widgetCreateParams)
    {
this._rawBodyData = new(widgetCreateParams._rawBodyData);
    }
#pragma warning restore CS8618

    public WidgetCreateParams(
IReadOnlyDictionary<string, JsonElement> rawHeaderData,
IReadOnlyDictionary<string, JsonElement> rawQueryData,
IReadOnlyDictionary<string, JsonElement> rawBodyData
    )
    {
this._rawHeaderData = new(rawHeaderData);
this._rawQueryData = new(rawQueryData);
this._rawBodyData = new(rawBodyData);
    }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    WidgetCreateParams(
FrozenDictionary<string, JsonElement> rawHeaderData,
FrozenDictionary<string, JsonElement> rawQueryData,
FrozenDictionary<string, JsonElement> rawBodyData
    )
    {
this._rawHeaderData = new(rawHeaderData);
this._rawQueryData = new(rawQueryData);
this._rawBodyData = new(rawBodyData);
    }
#pragma warning restore CS8618

    /// <inheritdoc cref="IFromRawJson{T}.FromRawUnchecked"/>
    public static WidgetCreateParams FromRawUnchecked(
IReadOnlyDictionary<string, JsonElement> rawHeaderData,
IReadOnlyDictionary<string, JsonElement> rawQueryData,
IReadOnlyDictionary<string, JsonElement> rawBodyData
    )
    {
        return new(
FrozenDictionary.ToFrozenDictionary(rawHeaderData),
FrozenDictionary.ToFrozenDictionary(rawQueryData),
FrozenDictionary.ToFrozenDictionary(rawBodyData)
        );
    }

    public override string ToString() =>
        JsonSerializer.Serialize(
            FriendlyJsonPrinter.PrintValue(
                new Dictionary<string, JsonElement>()
                {
["HeaderData"] = FriendlyJsonPrinter.PrintValue(JsonSerializer.SerializeToElement(this._rawHeaderData.Freeze())),
["QueryData"] = FriendlyJsonPrinter.PrintValue(JsonSerializer.SerializeToElement(this._rawQueryData.Freeze())),
["BodyData"] = FriendlyJsonPrinter.PrintValue(this._rawBodyData.Freeze()),
                }
            ),
            ModelBase.ToStringSerializerOptions
        );

    public virtual bool Equals(WidgetCreateParams? other)
    {
        if (other == null)
        {
            return false;
        }
        return this._rawHeaderData.Equals(other._rawHeaderData)
            && this._rawQueryData.Equals(other._rawQueryData)
            && this._rawBodyData.Equals(other._rawBodyData);
    }

    public override System::Uri Url(ClientOptions options)
    {
        return new System::UriBuilder(options.BaseUrl.ToString().TrimEnd('/') + "/v1/widgets")
        {
            Query = this.QueryString(options),
        }.Uri;
    }

    internal override HttpContent? BodyContent()
    {
        return new StringContent(
            JsonSerializer.Serialize(this.RawBodyData, ModelBase.SerializerOptions),
            Text::Encoding.UTF8,
            "application/json"
        );
    }

    internal override void AddHeadersToRequest(HttpRequestMessage request, ClientOptions options)
    {
        ParamsBase.AddDefaultHeaders(request, options);
        foreach (var item in this.RawHeaderData)
        {
            ParamsBase.AddHeaderElementToRequest(request, item.Key, item.Value);
        }
    }

    public override int GetHashCode()
    {
        return 0;
    }
}

[JsonConverter(typeof(ColorConverter))]
public enum Color
{
    Red,
    Green,
    Blue,
}

sealed class ColorConverter : JsonConverter<Color>
{
    public override Color Read(
        ref Utf8JsonReader reader,
        System::Type typeToConvert,
        JsonSerializerOptions options
    )
    {
        return JsonSerializer.Deserialize<string>(ref reader, options) switch
        {
"red" => Color.Red,
"green" => Color.Green,
"blue" => Color.Blue,
            _ => (Color)(-1),
        };
    }

    public override void Write(Utf8JsonWriter writer, Color value, JsonSerializerOptions options)
    {
        JsonSerializer.Serialize(
            writer,
            value switch
            {
Color.Red => "red",
Color.Green => "green",
Color.Blue => "blue",
                _ => throw new AcmeInvalidDataException(
                    string.Format("Invalid value '{0}' in {1}", value, nameof(value))
                ),
            },
            options
        );
    }
}

[JsonConverter(typeof(JsonModelConverter<Dimensions, DimensionsFromRaw>))]
public sealed record class Dimensions : JsonModel
{
    public required double Width
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNotNullStruct<double>("width");
        }
init { this._rawData.Set("width", value); }
    }

    public double? Height
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNullableStruct<double>("height");
        }
init
{
if (value == null)
{
return;
}

this._rawData.Set("height", value);
}
    }

    /// <inheritdoc/>
    public override void Validate()
    {
_ = this.Width;
_ = this.Height;
    }

    public Dimensions() { }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    public Dimensions(Dimensions dimensions)
        : base(dimensions) { }
#pragma warning restore CS8618

    public Dimensions(IReadOnlyDictionary<string, JsonElement> rawData)
    {
        this._rawData = new(rawData);
    }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    Dimensions(FrozenDictionary<string, JsonElement> rawData)
    {
        this._rawData = new(rawData);
    }
#pragma warning restore CS8618

    /// <inheritdoc cref="DimensionsFromRaw.FromRawUnchecked"/>
    public static Dimensions FromRawUnchecked(IReadOnlyDictionary<string, JsonElement> rawData)
    {
        return new(FrozenDictionary.ToFrozenDictionary(rawData));
    }

    [SetsRequiredMembers]
    public Dimensions(double width)
        : this()
    {
        this.Width = width;
    }
}

class DimensionsFromRaw : IFromRawJson<Dimensions>
{
    /// <inheritdoc/>
    public Dimensions FromRawUnchecked(IReadOnlyDictionary<string, JsonElement> rawData) =>
        Dimensions.FromRawUnchecked(rawData);
}
