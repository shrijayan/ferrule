# typed: strong

module Acme
  module Models
    class ErrorInfo < Acme::Internal::Type::BaseModel
      OrHash = T.type_alias { T.any(Acme::ErrorInfo, Acme::Internal::AnyHash) }

      sig { returns(T.nilable(String)) }
      attr_reader :code

      sig { params(code: String).void }
      attr_writer :code

      sig { returns(T.nilable(String)) }
      attr_reader :message

      sig { params(message: String).void }
      attr_writer :message

      sig { params(code: String, message: String).returns(T.attached_class) }
      def self.new(code: nil, message: nil)
      end

      sig { override.returns({ code: String, message: String }) }
      def to_hash
      end
    end
  end
end
