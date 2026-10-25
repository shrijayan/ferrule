# typed: strong

module Acme
  module Models
    class WidgetCreateParams < Acme::Internal::Type::BaseModel
      extend Acme::Internal::Type::RequestParameters::Converter
      include Acme::Internal::Type::RequestParameters

      OrHash = T.type_alias { T.any(Acme::WidgetCreateParams, Acme::Internal::AnyHash) }

      # The widget's display name.
      sig { returns(String) }
      attr_accessor :name

      sig { returns(T.nilable(Acme::WidgetCreateParams::Color::OrSymbol)) }
      attr_reader :color

      sig { params(color: Acme::WidgetCreateParams::Color::OrSymbol).void }
      attr_writer :color

      sig { returns(T.nilable(Acme::WidgetCreateParams::Dimensions)) }
      attr_reader :dimensions

      sig { params(dimensions: Acme::WidgetCreateParams::Dimensions::OrHash).void }
      attr_writer :dimensions

      sig { returns(T.nilable(T::Array[String])) }
      attr_reader :tags

      sig { params(tags: T::Array[String]).void }
      attr_writer :tags

      sig { params(name: String, color: Acme::WidgetCreateParams::Color::OrSymbol, dimensions: Acme::WidgetCreateParams::Dimensions::OrHash, tags: T::Array[String], request_options: Acme::RequestOptions::OrHash).returns(T.attached_class) }
      def self.new(
        # The widget's display name.
        name:,
        color: nil,
        dimensions: nil,
        tags: nil,
        request_options: {}
      )
      end

      sig { override.returns({ name: String, color: Acme::WidgetCreateParams::Color::OrSymbol, dimensions: Acme::WidgetCreateParams::Dimensions, tags: T::Array[String], request_options: Acme::RequestOptions }) }
      def to_hash
      end

      module Color
        extend Acme::Internal::Type::Enum

        TaggedSymbol = T.type_alias { T.all(Symbol, Acme::WidgetCreateParams::Color) }
        OrSymbol = T.type_alias { T.any(Symbol, String) }

        RED = T.let(:red, Acme::WidgetCreateParams::Color::TaggedSymbol)
        GREEN = T.let(:green, Acme::WidgetCreateParams::Color::TaggedSymbol)
        BLUE = T.let(:blue, Acme::WidgetCreateParams::Color::TaggedSymbol)

        sig { override.returns(T::Array[Acme::WidgetCreateParams::Color::TaggedSymbol]) }
        def self.values
        end
      end

      class Dimensions < Acme::Internal::Type::BaseModel
        OrHash = T.type_alias { T.any(Acme::WidgetCreateParams::Dimensions, Acme::Internal::AnyHash) }

        sig { returns(Float) }
        attr_accessor :width

        sig { returns(T.nilable(Float)) }
        attr_reader :height

        sig { params(height: Float).void }
        attr_writer :height

        sig { params(width: Float, height: Float).returns(T.attached_class) }
        def self.new(width:, height: nil)
        end

        sig { override.returns({ width: Float, height: Float }) }
        def to_hash
        end
      end
    end
  end
end
