using System.Collections.Frozen;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;
using System.Text.Json;
using System.Text.Json.Serialization;
using Acme.Core;

namespace Acme.Models;

/// <summary>
/// Labels by language.
/// </summary>
[JsonConverter(typeof(JsonModelConverter<LabelsItem, LabelsItemFromRaw>))]
public sealed record class LabelsItem : JsonModel
{
    public required string Text
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNotNullClass<string>("text");
        }
init { this._rawData.Set("text", value); }
    }

    public string? Color
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNullableClass<string>("color");
        }
init
{
if (value == null)
{
return;
}

this._rawData.Set("color", value);
}
    }

    /// <inheritdoc/>
    public override void Validate()
    {
_ = this.Text;
_ = this.Color;
    }

    public LabelsItem() { }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    public LabelsItem(LabelsItem labelsItem)
        : base(labelsItem) { }
#pragma warning restore CS8618

    public LabelsItem(IReadOnlyDictionary<string, JsonElement> rawData)
    {
        this._rawData = new(rawData);
    }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    LabelsItem(FrozenDictionary<string, JsonElement> rawData)
    {
        this._rawData = new(rawData);
    }
#pragma warning restore CS8618

    /// <inheritdoc cref="LabelsItemFromRaw.FromRawUnchecked"/>
    public static LabelsItem FromRawUnchecked(IReadOnlyDictionary<string, JsonElement> rawData)
    {
        return new(FrozenDictionary.ToFrozenDictionary(rawData));
    }

    [SetsRequiredMembers]
    public LabelsItem(string text)
        : this()
    {
        this.Text = text;
    }
}

class LabelsItemFromRaw : IFromRawJson<LabelsItem>
{
    /// <inheritdoc/>
    public LabelsItem FromRawUnchecked(IReadOnlyDictionary<string, JsonElement> rawData) =>
        LabelsItem.FromRawUnchecked(rawData);
}
