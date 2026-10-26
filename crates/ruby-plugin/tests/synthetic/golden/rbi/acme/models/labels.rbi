# typed: strong

module Acme
  module Models
    class LabelsItem < Acme::Internal::Type::BaseModel
      OrHash = T.type_alias { T.any(Acme::LabelsItem, Acme::Internal::AnyHash) }

      sig { returns(String) }
      attr_accessor :text

      sig { returns(T.nilable(String)) }
      attr_reader :color

      sig { params(color: String).void }
      attr_writer :color

      sig { params(text: String, color: String).returns(T.attached_class) }
      def self.new(text:, color: nil)
      end

      sig { override.returns({ text: String, color: String }) }
      def to_hash
      end
    end

    Labels = T.let(Acme::Internal::Type::HashOf[Acme::LabelsItem], Acme::Internal::Type::Converter)
  end
end
