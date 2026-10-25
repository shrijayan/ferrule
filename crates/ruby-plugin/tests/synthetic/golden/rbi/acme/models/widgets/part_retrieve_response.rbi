# typed: strong

module Acme
  module Models
    module Widgets
      class PartRetrieveResponse < Acme::Internal::Type::BaseModel
        OrHash = T.type_alias { T.any(Acme::Models::Widgets::PartRetrieveResponse, Acme::Internal::AnyHash) }

        sig { returns(String) }
        attr_accessor :id

        # The part's stock keeping unit.
        sig { returns(String) }
        attr_accessor :sku

        sig { returns(T.nilable(Acme::ErrorInfo)) }
        attr_reader :error

        sig { params(error: Acme::ErrorInfo::OrHash).void }
        attr_writer :error

        sig { params(id: String, sku: String, error: Acme::ErrorInfo::OrHash).returns(T.attached_class) }
        def self.new(
          id:,
          # The part's stock keeping unit.
          sku:,
          error: nil
        )
        end

        sig { override.returns({ id: String, sku: String, error: Acme::ErrorInfo }) }
        def to_hash
        end
      end
    end
  end
end
