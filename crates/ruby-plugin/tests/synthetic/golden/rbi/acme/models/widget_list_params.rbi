# typed: strong

module Acme
  module Models
    class WidgetListParams < Acme::Internal::Type::BaseModel
      extend Acme::Internal::Type::RequestParameters::Converter
      include Acme::Internal::Type::RequestParameters

      OrHash = T.type_alias { T.any(Acme::WidgetListParams, Acme::Internal::AnyHash) }

      sig { returns(T.nilable(T::Array[String])) }
      attr_reader :colors

      sig { params(colors: T::Array[String]).void }
      attr_writer :colors

      sig { returns(T.nilable(Time)) }
      attr_reader :created_after

      sig { params(created_after: Time).void }
      attr_writer :created_after

      sig { returns(T.nilable(Integer)) }
      attr_reader :limit

      sig { params(limit: Integer).void }
      attr_writer :limit

      sig { params(colors: T::Array[String], created_after: Time, limit: Integer, request_options: Acme::RequestOptions::OrHash).returns(T.attached_class) }
      def self.new(colors: nil, created_after: nil, limit: nil, request_options: {})
      end

      sig { override.returns({ colors: T::Array[String], created_after: Time, limit: Integer, request_options: Acme::RequestOptions }) }
      def to_hash
      end
    end
  end
end
