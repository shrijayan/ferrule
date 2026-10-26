# typed: strong

module Acme
  module Models
    class WidgetDeleteParams < Acme::Internal::Type::BaseModel
      extend Acme::Internal::Type::RequestParameters::Converter
      include Acme::Internal::Type::RequestParameters

      OrHash = T.type_alias { T.any(Acme::WidgetDeleteParams, Acme::Internal::AnyHash) }

      sig { returns(String) }
      attr_accessor :widget_id

      sig { params(widget_id: String, request_options: Acme::RequestOptions::OrHash).returns(T.attached_class) }
      def self.new(widget_id:, request_options: {})
      end

      sig { override.returns({ widget_id: String, request_options: Acme::RequestOptions }) }
      def to_hash
      end
    end
  end
end
