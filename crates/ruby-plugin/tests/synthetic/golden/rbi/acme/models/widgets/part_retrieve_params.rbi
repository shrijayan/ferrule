# typed: strong

module Acme
  module Models
    module Widgets
      class PartRetrieveParams < Acme::Internal::Type::BaseModel
        extend Acme::Internal::Type::RequestParameters::Converter
        include Acme::Internal::Type::RequestParameters

        OrHash = T.type_alias { T.any(Acme::Widgets::PartRetrieveParams, Acme::Internal::AnyHash) }

        sig { returns(String) }
        attr_accessor :widget_id

        sig { returns(Integer) }
        attr_accessor :part_number

        sig { params(widget_id: String, part_number: Integer, request_options: Acme::RequestOptions::OrHash).returns(T.attached_class) }
        def self.new(widget_id:, part_number:, request_options: {})
        end

        sig { override.returns({ widget_id: String, part_number: Integer, request_options: Acme::RequestOptions }) }
        def to_hash
        end
      end
    end
  end
end
