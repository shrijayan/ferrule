using System.Collections.Frozen;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Diagnostics.CodeAnalysis;
using System.Text.Json;
using System.Text.Json.Serialization;
using Acme.Core;
using Acme.Exceptions;
using System = System;

namespace Acme.Models.Widgets;

/// <summary>
/// A widget in the catalogue.
/// </summary>
[JsonConverter(typeof(JsonModelConverter<Widget, WidgetFromRaw>))]
public sealed record class Widget : JsonModel
{
    public required string ID
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNotNullClass<string>("id");
        }
init { this._rawData.Set("id", value); }
    }

    public required System::DateTimeOffset CreatedAt
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNotNullStruct<System::DateTimeOffset>("created_at");
        }
init { this._rawData.Set("created_at", value); }
    }

    public required string Name
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNotNullClass<string>("name");
        }
init { this._rawData.Set("name", value); }
    }

    /// <summary>
    /// Where the widget is in its life cycle.
    /// </summary>
    public required ApiEnum<string, Status> Status
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNotNullClass<ApiEnum<string, Status>>("status");
        }
init { this._rawData.Set("status", value); }
    }

    public string? HomepageUrl
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNullableClass<string>("homepage_url");
        }
init
{
if (value == null)
{
return;
}

this._rawData.Set("homepage_url", value);
}
    }

    /// <summary>
    /// Labels by language.
    /// </summary>
    public IReadOnlyDictionary<string, LabelsItem>? Labels
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNullableClass<FrozenDictionary<string, LabelsItem>>("labels");
        }
init
{
if (value == null)
{
return;
}

this._rawData.Set<FrozenDictionary<string, LabelsItem>?>("labels", value == null ? null : FrozenDictionary.ToFrozenDictionary(value));
}
    }

    public IReadOnlyList<Part>? Parts
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNullableStruct<ImmutableArray<Part>>("parts");
        }
init
{
if (value == null)
{
return;
}

this._rawData.Set<ImmutableArray<Part>?>("parts", value == null ? null : ImmutableArray.ToImmutableArray(value));
}
    }

    public string? SKU
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNullableClass<string>("sku");
        }
init { this._rawData.Set("sku", value); }
    }

    public double? WeightKg
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNullableStruct<double>("weight_kg");
        }
init
{
if (value == null)
{
return;
}

this._rawData.Set("weight_kg", value);
}
    }

    /// <inheritdoc/>
    public override void Validate()
    {
_ = this.ID;
_ = this.CreatedAt;
_ = this.Name;
this.Status.Validate();
_ = this.HomepageUrl;
if (this.Labels != null)
{
foreach (var item in this.Labels.Values)
{
item.Validate();
}
}
foreach (var item in this.Parts ?? [])
{
item.Validate();
}
_ = this.SKU;
_ = this.WeightKg;
    }

    public Widget() { }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    public Widget(Widget widget)
        : base(widget) { }
#pragma warning restore CS8618

    public Widget(IReadOnlyDictionary<string, JsonElement> rawData)
    {
        this._rawData = new(rawData);
    }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    Widget(FrozenDictionary<string, JsonElement> rawData)
    {
        this._rawData = new(rawData);
    }
#pragma warning restore CS8618

    /// <inheritdoc cref="WidgetFromRaw.FromRawUnchecked"/>
    public static Widget FromRawUnchecked(IReadOnlyDictionary<string, JsonElement> rawData)
    {
        return new(FrozenDictionary.ToFrozenDictionary(rawData));
    }
}

class WidgetFromRaw : IFromRawJson<Widget>
{
    /// <inheritdoc/>
    public Widget FromRawUnchecked(IReadOnlyDictionary<string, JsonElement> rawData) =>
        Widget.FromRawUnchecked(rawData);
}

[JsonConverter(typeof(JsonModelConverter<Part, PartFromRaw>))]
public sealed record class Part : JsonModel
{
    public string? Label
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNullableClass<string>("label");
        }
init
{
if (value == null)
{
return;
}

this._rawData.Set("label", value);
}
    }

    public long? PartNumber
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNullableStruct<long>("part_number");
        }
init
{
if (value == null)
{
return;
}

this._rawData.Set("part_number", value);
}
    }

    /// <inheritdoc/>
    public override void Validate()
    {
_ = this.Label;
_ = this.PartNumber;
    }

    public Part() { }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    public Part(Part part)
        : base(part) { }
#pragma warning restore CS8618

    public Part(IReadOnlyDictionary<string, JsonElement> rawData)
    {
        this._rawData = new(rawData);
    }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    Part(FrozenDictionary<string, JsonElement> rawData)
    {
        this._rawData = new(rawData);
    }
#pragma warning restore CS8618

    /// <inheritdoc cref="PartFromRaw.FromRawUnchecked"/>
    public static Part FromRawUnchecked(IReadOnlyDictionary<string, JsonElement> rawData)
    {
        return new(FrozenDictionary.ToFrozenDictionary(rawData));
    }
}

class PartFromRaw : IFromRawJson<Part>
{
    /// <inheritdoc/>
    public Part FromRawUnchecked(IReadOnlyDictionary<string, JsonElement> rawData) =>
        Part.FromRawUnchecked(rawData);
}

/// <summary>
/// Where the widget is in its life cycle.
/// </summary>
[JsonConverter(typeof(StatusConverter))]
public enum Status
{
    Draft,
    Active,
    Retired,
}

sealed class StatusConverter : JsonConverter<Status>
{
    public override Status Read(
        ref Utf8JsonReader reader,
        System::Type typeToConvert,
        JsonSerializerOptions options
    )
    {
        return JsonSerializer.Deserialize<string>(ref reader, options) switch
        {
"draft" => Status.Draft,
"active" => Status.Active,
"retired" => Status.Retired,
            _ => (Status)(-1),
        };
    }

    public override void Write(Utf8JsonWriter writer, Status value, JsonSerializerOptions options)
    {
        JsonSerializer.Serialize(
            writer,
            value switch
            {
Status.Draft => "draft",
Status.Active => "active",
Status.Retired => "retired",
                _ => throw new AcmeInvalidDataException(
                    string.Format("Invalid value '{0}' in {1}", value, nameof(value))
                ),
            },
            options
        );
    }
}
